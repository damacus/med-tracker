package io.damacus.medtracker.notifications

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.ScheduleDto
import java.time.Clock
import java.time.LocalTime
import java.time.ZonedDateTime
import java.util.Locale

data class ReminderTimes(
    val morning: LocalTime = LocalTime.of(8, 0),
    val afternoon: LocalTime = LocalTime.of(13, 0),
    val evening: LocalTime = LocalTime.of(18, 0),
    val night: LocalTime = LocalTime.of(21, 0),
    val defaultTime: LocalTime = LocalTime.of(9, 0)
) {
    fun timeForCycle(doseCycle: String?): LocalTime = when (doseCycle?.trim()?.lowercase(Locale.ROOT)) {
        "morning" -> morning
        "afternoon" -> afternoon
        "evening" -> evening
        "night", "bedtime" -> night
        else -> defaultTime
    }

    companion object {
        val DEFAULT = ReminderTimes()

        fun fromStrings(
            morning: String?,
            afternoon: String?,
            evening: String?,
            night: String?
        ): ReminderTimes = ReminderTimes(
            morning = parseTime(morning) ?: DEFAULT.morning,
            afternoon = parseTime(afternoon) ?: DEFAULT.afternoon,
            evening = parseTime(evening) ?: DEFAULT.evening,
            night = parseTime(night) ?: DEFAULT.night
        )

        private fun parseTime(raw: String?): LocalTime? {
            if (raw.isNullOrBlank()) return null
            return runCatching { LocalTime.parse(raw.trim()) }.getOrNull()
        }
    }
}

data class ScheduledReminder(
    val scheduleId: Long,
    val medicationName: String,
    val personName: String?,
    val doseSummary: String,
    val doseCycle: String,
    val triggerAtMillis: Long,
    val requestCode: Int = requestCodeFor(scheduleId)
) {
    companion object {
        fun requestCodeFor(scheduleId: Long): Int = (scheduleId xor (scheduleId ushr 32)).toInt()
    }
}

interface ReminderAlarmBackend {
    fun schedule(reminder: ScheduledReminder)
    fun cancel(requestCode: Int, scheduleId: Long)
}

class AndroidReminderAlarmBackend(
    private val context: Context
) : ReminderAlarmBackend {
    private val alarmManager: AlarmManager?
        get() = context.getSystemService(AlarmManager::class.java)

    override fun schedule(reminder: ScheduledReminder) {
        val manager = alarmManager ?: return
        val intent = Intent(context, ReminderBroadcastReceiver::class.java).apply {
            action = ReminderBroadcastReceiver.ACTION_MEDICATION_REMINDER
            putExtra(ReminderBroadcastReceiver.EXTRA_SCHEDULE_ID, reminder.scheduleId)
            putExtra(ReminderBroadcastReceiver.EXTRA_MEDICATION_NAME, reminder.medicationName)
            putExtra(ReminderBroadcastReceiver.EXTRA_PERSON_NAME, reminder.personName)
            putExtra(ReminderBroadcastReceiver.EXTRA_DOSE_SUMMARY, reminder.doseSummary)
            putExtra(ReminderBroadcastReceiver.EXTRA_DOSE_CYCLE, reminder.doseCycle)
        }
        val pendingIntent = PendingIntent.getBroadcast(
            context,
            reminder.requestCode,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        manager.setAndAllowWhileIdle(
            AlarmManager.RTC_WAKEUP,
            reminder.triggerAtMillis,
            pendingIntent
        )
    }

    override fun cancel(requestCode: Int, scheduleId: Long) {
        val manager = alarmManager ?: return
        val intent = Intent(context, ReminderBroadcastReceiver::class.java).apply {
            action = ReminderBroadcastReceiver.ACTION_MEDICATION_REMINDER
            putExtra(ReminderBroadcastReceiver.EXTRA_SCHEDULE_ID, scheduleId)
        }
        val existing = PendingIntent.getBroadcast(
            context,
            requestCode,
            intent,
            PendingIntent.FLAG_NO_CREATE or PendingIntent.FLAG_IMMUTABLE
        ) ?: return
        manager.cancel(existing)
        existing.cancel()
    }
}

class ReminderScheduler(
    private val backend: ReminderAlarmBackend,
    private val clock: Clock = Clock.systemDefaultZone()
) {
    constructor(context: Context) : this(AndroidReminderAlarmBackend(context.applicationContext))

    private val lock = Any()
    private val scheduledIds = mutableSetOf<Long>()

    fun syncSchedules(
        schedules: List<ScheduleDto>,
        medications: List<MedicationDto>,
        people: List<PersonDto>,
        remindersEnabled: Boolean = true,
        customTimes: ReminderTimes = ReminderTimes.DEFAULT
    ): List<ScheduledReminder> = synchronized(lock) {
        if (!remindersEnabled) {
            cancelAllLocked()
            return@synchronized emptyList()
        }

        val medicationMap = medications.associateBy { it.id }
        val peopleMap = people.associateBy { it.id }
        val activeSchedules = schedules.filter { it.id != null && it.active && !it.paused }
        val activeIds = activeSchedules.mapNotNull { it.id }.toSet()

        val staleIds = scheduledIds - activeIds
        staleIds.forEach { scheduleId ->
            backend.cancel(ScheduledReminder.requestCodeFor(scheduleId), scheduleId)
        }
        scheduledIds.removeAll(staleIds)

        val now = ZonedDateTime.now(clock)
        val scheduled = activeSchedules.mapNotNull { schedule ->
            val scheduleId = schedule.id ?: return@mapNotNull null
            val medication = schedule.medicationId?.let { medicationMap[it] }
            val person = schedule.personId?.let { peopleMap[it] }
            val medicationName = medication?.displayName?.takeIf { it.isNotBlank() }
                ?: medication?.name?.takeIf { it.isNotBlank() }
                ?: "Medication"
            val doseSummary = formatDose(schedule.doseAmount, schedule.doseUnit)
            val cycle = schedule.doseCycle?.takeIf { it.isNotBlank() } ?: "scheduled"
            val targetTime = customTimes.timeForCycle(schedule.doseCycle)
            val triggerAtMillis = nextTriggerMillis(now, targetTime)

            val reminder = ScheduledReminder(
                scheduleId = scheduleId,
                medicationName = medicationName,
                personName = person?.name,
                doseSummary = doseSummary,
                doseCycle = cycle,
                triggerAtMillis = triggerAtMillis
            )
            backend.schedule(reminder)
            scheduledIds.add(scheduleId)
            reminder
        }

        scheduled
    }

    fun cancelSchedule(scheduleId: Long) = synchronized(lock) {
        backend.cancel(ScheduledReminder.requestCodeFor(scheduleId), scheduleId)
        scheduledIds.remove(scheduleId)
    }

    fun cancelAll() = synchronized(lock) {
        cancelAllLocked()
    }

    private fun cancelAllLocked() {
        scheduledIds.forEach { scheduleId ->
            backend.cancel(ScheduledReminder.requestCodeFor(scheduleId), scheduleId)
        }
        scheduledIds.clear()
    }

    private fun nextTriggerMillis(now: ZonedDateTime, targetTime: LocalTime): Long {
        val todayTarget = now.with(targetTime).withNano(0)
        val next = if (todayTarget.isAfter(now)) todayTarget else todayTarget.plusDays(1)
        return next.toInstant().toEpochMilli()
    }

    private fun formatDose(amount: Double?, unit: String?): String {
        if (amount == null) return ""
        val formattedAmount = if (amount % 1.0 == 0.0) {
            amount.toLong().toString()
        } else {
            amount.toString()
        }
        return if (unit.isNullOrBlank()) formattedAmount else "$formattedAmount ${unit.trim()}"
    }
}
