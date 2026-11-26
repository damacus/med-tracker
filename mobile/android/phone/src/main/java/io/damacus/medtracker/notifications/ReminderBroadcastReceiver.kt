package io.damacus.medtracker.notifications

import android.Manifest
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import io.damacus.medtracker.R
import io.damacus.medtracker.MedTrackerApplication

data class ReminderNotificationContent(
    val title: String,
    val body: String
)

class ReminderBroadcastReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val application = context.applicationContext as MedTrackerApplication
        val owner = application.sessionManager.sessionState.value.owner ?: return
        val scheduleId = intent.getLongExtra(EXTRA_SCHEDULE_ID, 0L)
        val occurrenceKey = intent.getStringExtra(EXTRA_OCCURRENCE_KEY)
        val triggerAtMillis = intent.getLongExtra(EXTRA_TRIGGER_AT_MILLIS, -1L).takeIf { it > 0L }
        val scheduler = application.reminderScheduler
        val reminder = scheduler.reminderForDelivery(owner, scheduleId, occurrenceKey, triggerAtMillis) ?: return

        handleReminderDelivery(
            scheduleId = scheduleId,
            occurrenceKey = occurrenceKey,
            triggerAtMillis = triggerAtMillis,
            scheduler = scheduler
        )

        NotificationChannels.registerAll(context)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            return
        }

        val content = buildNotificationContent(
            medicationName = reminder.medicationName,
            personName = reminder.personName,
            doseSummary = "",
            doseCycle = reminder.doseCycle
        )

        val launchIntent = context.packageManager.getLaunchIntentForPackage(context.packageName)?.apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val contentPendingIntent = launchIntent?.let {
            PendingIntent.getActivity(
                context,
                ScheduledReminder.requestCodeFor(scheduleId),
                it,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
        }

        val notification = NotificationCompat.Builder(context, NotificationChannels.MEDICATION_REMINDERS_CHANNEL_ID)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(content.title)
            .setContentText(content.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(content.body))
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setAutoCancel(true)
            .apply {
                if (contentPendingIntent != null) {
                    setContentIntent(contentPendingIntent)
                }
            }
            .build()

        NotificationManagerCompat.from(context).notify(
            ScheduledReminder.requestCodeFor(scheduleId),
            notification
        )
    }

    companion object {
        const val ACTION_MEDICATION_REMINDER = "io.damacus.medtracker.action.MEDICATION_REMINDER"
        const val EXTRA_SCHEDULE_ID = "extra_schedule_id"
        const val EXTRA_OCCURRENCE_KEY = "extra_occurrence_key"
        const val EXTRA_TRIGGER_AT_MILLIS = "extra_trigger_at_millis"

        internal fun handleReminderDelivery(
            scheduleId: Long,
            occurrenceKey: String?,
            triggerAtMillis: Long?,
            scheduler: ReminderScheduler
        ): ScheduledReminder? = scheduler.scheduleNextAfterDelivery(
            deliveredScheduleId = scheduleId,
            deliveredOccurrenceKey = occurrenceKey,
            deliveredTriggerAtMillis = triggerAtMillis
        )

        fun buildNotificationContent(
            medicationName: String,
            personName: String?,
            doseSummary: String,
            doseCycle: String?
        ): ReminderNotificationContent {
            val title = "Time for $medicationName"
            val parts = buildList {
                if (!personName.isNullOrBlank()) {
                    add("For $personName")
                }
                if (doseSummary.isNotBlank()) {
                    add("Dose: $doseSummary")
                }
                if (!doseCycle.isNullOrBlank()) {
                    add("(${doseCycle.trim()} reminder)")
                }
            }
            val body = if (parts.isEmpty()) {
                "Scheduled dose is due now."
            } else {
                parts.joinToString(" • ")
            }
            return ReminderNotificationContent(title = title, body = body)
        }
    }
}
