package io.damacus.medtracker.notifications

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context

object NotificationChannels {
    const val MEDICATION_REMINDERS_CHANNEL_ID = "medication_reminders"
    const val MEDICATION_REMINDERS_CHANNEL_NAME = "Medication Reminders"
    const val MEDICATION_REMINDERS_CHANNEL_DESCRIPTION = "Scheduled reminders when medication doses are due"

    fun registerAll(context: Context) {
        val notificationManager = context.getSystemService(NotificationManager::class.java) ?: return
        val remindersChannel = NotificationChannel(
            MEDICATION_REMINDERS_CHANNEL_ID,
            MEDICATION_REMINDERS_CHANNEL_NAME,
            NotificationManager.IMPORTANCE_HIGH
        ).apply {
            description = MEDICATION_REMINDERS_CHANNEL_DESCRIPTION
            enableVibration(true)
        }
        notificationManager.createNotificationChannel(remindersChannel)
    }
}
