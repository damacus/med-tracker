package io.damacus.medtracker.ui

import android.os.Build
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.BuildConfig
import io.damacus.medtracker.data.model.OidcExchangeRequest
import io.damacus.medtracker.data.model.SessionPayload
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class MainUiState(
    val isLoading: Boolean = false,
    val isLoggingOut: Boolean = false,
    val errorMessage: String? = null
)

class MainViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    val sessionState: StateFlow<AppSession> = sessionManager.sessionState

    private val _uiState = MutableStateFlow(MainUiState())
    val uiState: StateFlow<MainUiState> = _uiState.asStateFlow()

    fun exchangeOidc(idToken: String, nonce: String, codeVerifier: String) {
        authenticate(BuildConfig.SERVER_URL) { api ->
            api.exchangeOidc(
                BuildConfig.SERVER_URL,
                OidcExchangeRequest(
                    idToken = idToken,
                    nonce = nonce,
                    codeVerifier = codeVerifier,
                    deviceName = deviceName()
                )
            )
        }
    }

    fun authenticate(
        serverUrl: String,
        request: suspend (MedTrackerApi) -> ApiResult<SessionPayload>
    ) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            when (val result = request(apiClient)) {
                is ApiResult.Success -> {
                    sessionManager.saveSession(result.data, serverUrl)
                    _uiState.update { it.copy(isLoading = false, errorMessage = null) }
                }
                is ApiResult.Error -> {
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            errorMessage = result.message
                        )
                    }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            errorMessage = "Network connection error: ${result.cause.localizedMessage ?: "Unable to connect to server"}"
                        )
                    }
                }
            }
        }
    }

    fun deviceName(): String =
        "${Build.MANUFACTURER.replaceFirstChar { it.uppercase() }} ${Build.MODEL} (Android)"

    fun reportAuthenticationError(message: String) {
        _uiState.update { it.copy(isLoading = false, errorMessage = message) }
    }

    fun logout(sessionRevision: String = sessionState.value.revision) {
        val currentSession = sessionState.value
        if (sessionRevision != currentSession.revision) return
        sessionManager.clearSession()
        _uiState.value = MainUiState()
        val token = currentSession.accessToken
        if (token.isNullOrBlank()) return
        viewModelScope.launch {
            try {
                apiClient.logout(currentSession.serverUrl, token)
            } catch (error: CancellationException) {
                throw error
            } catch (_: Exception) {
                Unit
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return MainViewModel(sessionManager) as T
        }
    }
}
