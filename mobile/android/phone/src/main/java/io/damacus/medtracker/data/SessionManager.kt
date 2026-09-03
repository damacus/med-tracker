package io.damacus.medtracker.data

import android.content.Context
import io.damacus.medtracker.BuildConfig
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

@Serializable
data class AppSession(
    val serverUrl: String,
    val sessionPayload: SessionPayload?
) {
    val isLoggedIn: Boolean get() = sessionPayload?.accessToken?.isNotBlank() == true
    val accessToken: String? get() = sessionPayload?.accessToken
    val user: UserDto? get() = sessionPayload?.me
    val household: HouseholdDto? get() = sessionPayload?.household
}

class SessionManager(
    context: Context,
    private val credentialStore: CredentialStore = AndroidCredentialStore(context)
) {
    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }
    private val _sessionState = MutableStateFlow(loadSession())
    val sessionState: StateFlow<AppSession> = _sessionState.asStateFlow()

    private fun loadSession(): AppSession = runCatching {
        credentialStore.read()?.let { json.decodeFromString<AppSession>(it) }
    }.getOrNull()
        ?: AppSession(serverUrl = BuildConfig.SERVER_URL, sessionPayload = null)

    fun saveSession(payload: SessionPayload, serverUrl: String = BuildConfig.SERVER_URL) {
        val cleanUrl = if (!serverUrl.endsWith("/")) "$serverUrl/" else serverUrl
        val session = AppSession(serverUrl = cleanUrl, sessionPayload = payload)
        credentialStore.write(json.encodeToString(session))
        _sessionState.value = session
    }

    fun clearSession() {
        credentialStore.clear()
        _sessionState.value = AppSession(
            serverUrl = BuildConfig.SERVER_URL,
            sessionPayload = null
        )
    }
}
