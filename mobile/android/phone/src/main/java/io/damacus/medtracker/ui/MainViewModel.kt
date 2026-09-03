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
import io.damacus.medtracker.data.model.LoginRequest
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

    fun updateServerUrl(url: String) {
        sessionManager.updateServerUrl(url)
    }

    fun login(email: String, password: String, serverUrl: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            sessionManager.updateServerUrl(serverUrl)

            val deviceName = "${Build.MANUFACTURER.replaceFirstChar { it.uppercase() }} ${Build.MODEL} (Android)"
            val request = LoginRequest(
                email = email,
                password = password,
                deviceName = deviceName
            )

            when (val result = apiClient.login(serverUrl, request)) {
                is ApiResult.Success -> {
                    sessionManager.saveSession(result.data)
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

    fun logout() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoggingOut = true) }
            val currentSession = sessionState.value
            val token = currentSession.accessToken
            if (!token.isNullOrBlank()) {
                apiClient.logout(currentSession.serverUrl, token)
            }
            sessionManager.clearSession()
            _uiState.update { it.copy(isLoggingOut = false) }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return MainViewModel(sessionManager) as T
        }
    }
}
