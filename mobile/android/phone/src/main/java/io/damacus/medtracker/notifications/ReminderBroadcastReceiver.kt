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

data class ReminderNotificationContent(
    val title: String,
    val body: String
)

class ReminderBroadcastReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val medicationName = intent.getStringExtra(EXTRA_MEDICATION_NAME)?.takeIf { it.isNotBlank() } ?: return
        val scheduleId = intent.getLongExtra(EXTRA_SCHEDULE_ID, 0L)
        val personName = intent.getStringExtra(EXTRA_PERSON_NAME)
        val doseSummary = intent.getStringExtra(EXTRA_DOSE_SUMMARY).orEmpty()
        val doseCycle = intent.getStringExtra(EXTRA_DOSE_CYCLE)

        NotificationChannels.registerAll(context)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            return
        }

        val content = buildNotificationContent(
            medicationName = medicationName,
            personName = personName,
            doseSummary = doseSummary,
            doseCycle = doseCycle
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
        const val EXTRA_MEDICATION_NAME = "extra_medication_name"
        const val EXTRA_PERSON_NAME = "extra_person_name"
        const val EXTRA_DOSE_SUMMARY = "extra_dose_summary"
        const val EXTRA_DOSE_CYCLE = "extra_dose_cycle"

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
