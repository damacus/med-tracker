package io.damacus.medtracker.notifications

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import io.damacus.medtracker.data.api.DoseOccurrenceDto
import io.damacus.medtracker.data.SessionOwner
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.ScheduleDto
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.time.Clock
import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime
import java.time.OffsetDateTime
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

@Serializable
data class ScheduledReminder(
    val scheduleId: Long,
    val medicationName: String,
    val personName: String?,
    val doseSummary: String,
    val doseCycle: String,
    val triggerAtMillis: Long,
    val requestCode: Int = requestCodeFor(scheduleId),
    val occurrenceKey: String? = null,
    val owner: SessionOwner? = null
) {
    companion object {
        fun requestCodeFor(scheduleId: Long): Int = (scheduleId xor (scheduleId ushr 32)).toInt()
    }
}

interface ReminderOccurrenceStore {
    fun load(): List<ScheduledReminder>
    fun save(reminders: List<ScheduledReminder>)
}

class InMemoryReminderOccurrenceStore(
    initial: List<ScheduledReminder> = emptyList()
) : ReminderOccurrenceStore {
    private var snapshot: List<ScheduledReminder> = initial

    override fun load(): List<ScheduledReminder> = snapshot

    override fun save(reminders: List<ScheduledReminder>) {
        snapshot = reminders
    }
}

class FileReminderOccurrenceStore(
    private val file: File,
    private val json: Json = defaultJson
) : ReminderOccurrenceStore {

    override fun load(): List<ScheduledReminder> {
        if (!file.exists()) return emptyList()
        return runCatching {
            val content = file.readText()
            if (content.isBlank()) {
                emptyList()
            } else {
                json.decodeFromString<List<ScheduledReminder>>(content)
            }
        }.getOrElse { emptyList() }
    }

    override fun save(reminders: List<ScheduledReminder>) {
        val parent = file.parentFile
        if (parent != null && !parent.exists()) {
            parent.mkdirs()
        }
        val tmpFile = File(parent ?: file.absoluteFile.parentFile, "${file.name}.tmp")
        tmpFile.writeText(json.encodeToString(reminders))
        try {
            Files.move(
                tmpFile.toPath(),
                file.toPath(),
                StandardCopyOption.REPLACE_EXISTING,
                StandardCopyOption.ATOMIC_MOVE
            )
        } catch (_: AtomicMoveNotSupportedException) {
            Files.move(
                tmpFile.toPath(),
                file.toPath(),
                StandardCopyOption.REPLACE_EXISTING
            )
        }
    }

    companion object {
        const val OCCURRENCES_FILE_NAME = "reminder_occurrences.json"

        private val defaultJson = Json {
            ignoreUnknownKeys = true
            encodeDefaults = true
        }

        fun forContext(context: Context): FileReminderOccurrenceStore =
            FileReminderOccurrenceStore(File(context.noBackupFilesDir, OCCURRENCES_FILE_NAME))
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
            putExtra(ReminderBroadcastReceiver.EXTRA_OCCURRENCE_KEY, reminder.occurrenceKey)
            putExtra(ReminderBroadcastReceiver.EXTRA_TRIGGER_AT_MILLIS, reminder.triggerAtMillis)
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
    private val clock: Clock = Clock.systemDefaultZone(),
    private val store: ReminderOccurrenceStore = InMemoryReminderOccurrenceStore()
) {
    constructor(context: Context) : this(
        backend = AndroidReminderAlarmBackend(context.applicationContext),
        clock = Clock.systemDefaultZone(),
        store = FileReminderOccurrenceStore.forContext(context.applicationContext)
    )

    private val lock = Any()
    private val scheduledIds = mutableSetOf<Long>()

    fun syncOccurrences(
        occurrences: List<DoseOccurrenceDto>,
        schedules: List<ScheduleDto>,
        medications: List<MedicationDto>,
        people: List<PersonDto>,
        remindersEnabled: Boolean = true,
        customTimes: ReminderTimes = ReminderTimes.DEFAULT,
        owner: SessionOwner? = null,
        failedScheduleIds: Set<Long>? = emptySet(),
        privateText: Boolean = false,
        preferencesConfirmed: Boolean = true
    ): List<ScheduledReminder> = synchronized(lock) {
        if (!remindersEnabled) {
            cancelAllLocked()
            return@synchronized emptyList()
        }

        val activeScheduleMap = schedules
            .filter { it.id != null && it.active && !it.paused }
            .associateBy { it.id!! }
        val medicationMap = medications.associateBy { it.id }
        val peopleMap = people.associateBy { it.id }
        val nowMillis = clock.millis()

        val upcomingReminders = occurrences.mapNotNull { occurrence ->
            if (!occurrence.outcome.equals("open", ignoreCase = true)) return@mapNotNull null
            val schedule = activeScheduleMap[occurrence.scheduleId] ?: return@mapNotNull null
            val triggerAtMillis = resolveOccurrenceTriggerMillis(occurrence, schedule, customTimes)
                ?: return@mapNotNull null
            if (triggerAtMillis <= nowMillis) return@mapNotNull null

            val medication = schedule.medicationId?.let { medicationMap[it] }
            val person = schedule.personId?.let { peopleMap[it] }
            val medicationName = medication?.displayName?.takeIf { it.isNotBlank() }
                ?: medication?.name?.takeIf { it.isNotBlank() }
                ?: "Medication"
            val cycle = schedule.doseCycle?.takeIf { it.isNotBlank() } ?: "scheduled"

            ScheduledReminder(
                scheduleId = occurrence.scheduleId,
                medicationName = medicationName,
                personName = person?.name,
                doseSummary = "",
                doseCycle = cycle,
                triggerAtMillis = triggerAtMillis,
                occurrenceKey = occurrence.key,
                owner = owner
            )
        }.sortedBy { it.triggerAtMillis }

        val previous = store.load()
        val retained = previous.filter {
            it.owner == owner && it.triggerAtMillis > nowMillis &&
                (failedScheduleIds == null || it.scheduleId in failedScheduleIds)
        }
        val refreshed = if (preferencesConfirmed) upcomingReminders else {
            val openKeys = occurrences.filter { it.outcome == "open" }.map { it.key }.toSet()
            previous.filter {
                it.owner == owner && it.occurrenceKey in openKeys && it.triggerAtMillis > nowMillis
            }
        }
        val reminders = (retained + refreshed).map { reminder ->
            if (privateText || !preferencesConfirmed) reminder.copy(medicationName = "Medication", personName = null, doseSummary = "", doseCycle = "")
            else reminder
        }.sortedBy { it.triggerAtMillis }
        val previouslyKnownIds = scheduledIds + previous.map { it.scheduleId }
        val nextBySchedule = reminders
            .groupBy { it.scheduleId }
            .mapValues { (_, list) -> list.first() }

        val staleIds = previouslyKnownIds - nextBySchedule.keys
        staleIds.forEach { scheduleId ->
            backend.cancel(ScheduledReminder.requestCodeFor(scheduleId), scheduleId)
        }
        scheduledIds.clear()
        store.save(reminders)

        val armed = nextBySchedule.values.sortedBy { it.triggerAtMillis }
        armed.forEach { reminder ->
            backend.schedule(reminder)
            scheduledIds.add(reminder.scheduleId)
        }
        armed
    }

    fun reminderForDelivery(owner: SessionOwner, id: Long, key: String?, trigger: Long?): ScheduledReminder? =
        synchronized(lock) {
            store.load().firstOrNull {
                it.owner == owner && it.scheduleId == id && it.occurrenceKey == key && it.triggerAtMillis == trigger
            }
        }

    fun scheduleNextAfterDelivery(
        deliveredScheduleId: Long,
        deliveredOccurrenceKey: String? = null,
        deliveredTriggerAtMillis: Long? = null
    ): ScheduledReminder? = synchronized(lock) {
        val cutoffMillis = maxOf(clock.millis(), deliveredTriggerAtMillis ?: 0L)
        val remaining = store.load()
            .filterNot { reminder ->
                val matchesDeliveredKey = deliveredOccurrenceKey != null && reminder.occurrenceKey == deliveredOccurrenceKey
                val matchesDeliveredSlot = reminder.scheduleId == deliveredScheduleId && reminder.triggerAtMillis <= cutoffMillis
                val isExpired = reminder.triggerAtMillis <= clock.millis()
                matchesDeliveredKey || matchesDeliveredSlot || isExpired
            }
            .sortedBy { it.triggerAtMillis }

        store.save(remaining)

        val nextForSchedule = remaining.firstOrNull { it.scheduleId == deliveredScheduleId }
        if (nextForSchedule != null) {
            backend.schedule(nextForSchedule)
            scheduledIds.add(nextForSchedule.scheduleId)
        } else {
            scheduledIds.remove(deliveredScheduleId)
        }
        nextForSchedule
    }

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

        val staleIds = (scheduledIds + store.load().map { it.scheduleId }) - activeIds
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

        store.save(scheduled)
        scheduled
    }

    fun cancelSchedule(scheduleId: Long) = synchronized(lock) {
        backend.cancel(ScheduledReminder.requestCodeFor(scheduleId), scheduleId)
        scheduledIds.remove(scheduleId)
        store.save(store.load().filterNot { it.scheduleId == scheduleId })
    }

    fun cancelAll() = synchronized(lock) {
        cancelAllLocked()
    }

    private fun cancelAllLocked() {
        val allIds = scheduledIds + store.load().map { it.scheduleId }
        allIds.forEach { scheduleId ->
            backend.cancel(ScheduledReminder.requestCodeFor(scheduleId), scheduleId)
        }
        scheduledIds.clear()
        store.save(emptyList())
    }

    private fun resolveOccurrenceTriggerMillis(
        occurrence: DoseOccurrenceDto,
        schedule: ScheduleDto,
        customTimes: ReminderTimes
    ): Long? {
        val scheduledAtRaw = occurrence.scheduledAt?.trim()?.takeIf { it.isNotBlank() }
        if (scheduledAtRaw != null) {
            return runCatching { OffsetDateTime.parse(scheduledAtRaw).toInstant().toEpochMilli() }
                .recoverCatching { Instant.parse(scheduledAtRaw).toEpochMilli() }
                .getOrNull()
        }

        val windowDateRaw = occurrence.windowStartsOn?.trim()?.takeIf { it.isNotBlank() } ?: return null
        val localDate = runCatching { LocalDate.parse(windowDateRaw) }.getOrNull() ?: return null
        val targetTime = customTimes.timeForCycle(schedule.doseCycle)
        return ZonedDateTime.of(localDate, targetTime, clock.zone).toInstant().toEpochMilli()
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
