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
import kotlinx.serialization.Transient
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.util.UUID

@Serializable
data class AppSession(
    val serverUrl: String,
    val sessionPayload: SessionPayload?,
    @Transient val revision: String = UUID.randomUUID().toString()
) {
    val isLoggedIn: Boolean get() = sessionPayload?.accessToken?.isNotBlank() == true
    val accessToken: String? get() = sessionPayload?.accessToken
    val user: UserDto? get() = sessionPayload?.me
    val household: HouseholdDto? get() = sessionPayload?.household
}

class SessionManager(
    private val credentialStore: CredentialStore
) {
    constructor(context: Context, credentialStore: CredentialStore = AndroidCredentialStore(context)) : this(credentialStore)

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }
    private val _sessionState = MutableStateFlow(loadSession())
    val sessionState: StateFlow<AppSession> = _sessionState.asStateFlow()
    private val sessionObservers = mutableSetOf<(AppSession) -> Unit>()

    @Synchronized
    internal fun observeSession(observer: (AppSession) -> Unit): AutoCloseable {
        sessionObservers.add(observer)
        observer(_sessionState.value)
        return AutoCloseable { synchronized(this) { sessionObservers.remove(observer) } }
    }

    private fun publishSession(session: AppSession) {
        _sessionState.value = session
        sessionObservers.toList().forEach { it(session) }
    }

    private fun loadSession(): AppSession = runCatching {
        credentialStore.read()?.let { json.decodeFromString<AppSession>(it) }
    }.getOrNull()
        ?: AppSession(serverUrl = BuildConfig.SERVER_URL, sessionPayload = null)

    @Synchronized
    fun saveSession(payload: SessionPayload, serverUrl: String = BuildConfig.SERVER_URL) {
        val cleanUrl = if (!serverUrl.endsWith("/")) "$serverUrl/" else serverUrl
        val session = AppSession(serverUrl = cleanUrl, sessionPayload = payload)
        credentialStore.write(json.encodeToString(session))
        publishSession(session)
    }

    @Synchronized
    fun clearSession() {
        credentialStore.clear()
        publishSession(AppSession(
            serverUrl = BuildConfig.SERVER_URL,
            sessionPayload = null
        ))
    }
}
