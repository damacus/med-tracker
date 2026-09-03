package io.damacus.medtracker.data

import android.content.Context
import android.content.SharedPreferences
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

data class AppSession(
    val serverUrl: String,
    val sessionPayload: SessionPayload?
) {
    val isLoggedIn: Boolean get() = sessionPayload?.accessToken?.isNotBlank() == true
    val accessToken: String? get() = sessionPayload?.accessToken
    val user: UserDto? get() = sessionPayload?.me
    val household: HouseholdDto? get() = sessionPayload?.household
}

class SessionManager(context: Context) {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }

    private val prefs: SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    private val _sessionState = MutableStateFlow(loadSession())
    val sessionState: StateFlow<AppSession> = _sessionState.asStateFlow()

    companion object {
        private const val PREFS_NAME = "med_tracker_session"
        private const val KEY_SERVER_URL = "key_server_url"
        private const val KEY_SESSION_PAYLOAD = "key_session_payload"
        const val DEFAULT_SERVER_URL = "https://med-tracker-canary.damacus.io/"
    }

    private fun loadSession(): AppSession {
        val serverUrl = prefs.getString(KEY_SERVER_URL, null) ?: DEFAULT_SERVER_URL
        val payloadJson = prefs.getString(KEY_SESSION_PAYLOAD, null)
        val payload = payloadJson?.let {
            runCatching { json.decodeFromString<SessionPayload>(it) }.getOrNull()
        }
        return AppSession(serverUrl = serverUrl, sessionPayload = payload)
    }

    fun updateServerUrl(url: String) {
        val cleanUrl = if (!url.endsWith("/")) "$url/" else url
        prefs.edit().putString(KEY_SERVER_URL, cleanUrl).apply()
        _sessionState.value = _sessionState.value.copy(serverUrl = cleanUrl)
    }

    fun saveSession(payload: SessionPayload) {
        val serialized = json.encodeToString(payload)
        prefs.edit().putString(KEY_SESSION_PAYLOAD, serialized).apply()
        _sessionState.value = _sessionState.value.copy(sessionPayload = payload)
    }

    fun clearSession() {
        prefs.edit().remove(KEY_SESSION_PAYLOAD).apply()
        _sessionState.value = _sessionState.value.copy(sessionPayload = null)
    }
}
