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
import io.damacus.medtracker.data.model.HouseholdSelection
import io.damacus.medtracker.data.model.HouseholdDto
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
    val errorMessage: String? = null,
    val householdSelection: HouseholdSelection? = null,
    val selectionServerUrl: String? = null
)

class MainViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    val sessionState: StateFlow<AppSession> = sessionManager.sessionState

    private val _uiState = MutableStateFlow(MainUiState())
    val uiState: StateFlow<MainUiState> = _uiState.asStateFlow()

    init {
        if (sessionState.value.sessionPayload?.oauthState != null && sessionState.value.household == null) showHouseholds()
    }

    fun completeMobileLogin(payload: SessionPayload, serverUrl: String) {
        sessionManager.saveSession(payload.copy(household = null), serverUrl)
        showHouseholds()
    }

    fun showHouseholds() {
        val session = sessionState.value
        val token = session.accessToken ?: return
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            val result = apiClient.getHouseholds(session.serverUrl, token)
            if (sessionState.value.revision != session.revision) return@launch
            when (result) {
                is ApiResult.Success -> {
                    _uiState.value = MainUiState(
                        householdSelection = HouseholdSelection(result.data),
                        selectionServerUrl = session.serverUrl
                    )
                    result.data.singleOrNull()?.let { selectHousehold(it.id) }
                }
                is ApiResult.Error -> reportAuthenticationError(result.message)
                is ApiResult.NetworkError -> reportAuthenticationError("Unable to load households. Check your connection and retry.")
            }
        }
    }

    fun selectHousehold(householdId: Long) {
        val selection = uiState.value.householdSelection ?: return
        val household = selection.households.singleOrNull { it.id == householdId } ?: return
        val session = sessionState.value
        val payload = session.sessionPayload ?: return
        sessionManager.saveSession(payload.copy(household = HouseholdDto(household.id, household.name)), session.serverUrl)
        _uiState.value = MainUiState()
    }

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
            } catch (_: Exception) {}
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return MainViewModel(sessionManager) as T
        }
    }
}
