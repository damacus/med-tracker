package io.damacus.medtracker

import android.app.Application
import io.damacus.medtracker.companion.CompanionPublisher
import io.damacus.medtracker.companion.PhoneDataLayer
import io.damacus.medtracker.data.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

class MedTrackerApplication : Application() {
    val sessionManager by lazy { SessionManager(this) }
    private val companionScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    override fun onCreate() {
        super.onCreate()
        companionScope.launch {
            CompanionPublisher(PhoneDataLayer(this@MedTrackerApplication), BuildConfig.VERSION_NAME)
                .observe(sessionManager.sessionState.map { it.isLoggedIn })
        }
    }
}
