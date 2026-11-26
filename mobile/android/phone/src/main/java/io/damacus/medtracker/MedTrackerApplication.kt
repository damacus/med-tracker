package io.damacus.medtracker

import android.app.Application
import io.damacus.medtracker.auth.MobileSessionAuthentication
import io.damacus.medtracker.companion.CompanionPublisher
import io.damacus.medtracker.companion.PhoneDataLayer
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.offline.OfflineQueueRepository
import io.damacus.medtracker.notifications.NotificationChannels
import io.damacus.medtracker.notifications.ReminderScheduler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

class MedTrackerApplication : Application() {
    val sessionManager by lazy { SessionManager(this) }
    val offlineQueueRepository by lazy { OfflineQueueRepository.createPersistent(this) }
    val reminderScheduler by lazy { ReminderScheduler(this) }
    private val companionScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    override fun onCreate() {
        super.onCreate()
        NotificationChannels.registerAll(this)
        MobileSessionAuthentication.install(this, sessionManager)
        companionScope.launch {
            CompanionPublisher(PhoneDataLayer(this@MedTrackerApplication), BuildConfig.VERSION_NAME)
                .observe(sessionManager.sessionState.map { it.isLoggedIn })
        }
    }
}
