package io.damacus.medtracker.ui.admin

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.HouseholdAdminSettingsDto
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class AdministrationUiState(
    val sessionRevision: String? = null,
    val isLoading: Boolean = false,
    val adminSettings: HouseholdAdminSettingsDto? = null,
    val errorMessage: String? = null
)

class AdministrationViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(AdministrationUiState())
    val uiState: StateFlow<AdministrationUiState> = _uiState.asStateFlow()
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            _uiState.value = AdministrationUiState(sessionRevision = session.revision)
            loadAdminSettings(session)
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        super.onCleared()
    }

    fun loadAdminSettings(session: AppSession = sessionManager.sessionState.value) {
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        _uiState.update { it.copy(isLoading = true, errorMessage = null) }

        viewModelScope.launch {
            when (val res = apiClient.getAdminSettings(session.serverUrl, token, householdId)) {
                is ApiResult.Success -> {
                    _uiState.update { it.copy(isLoading = false, adminSettings = res.data) }
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = res.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error loading admin settings") }
                }
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return AdministrationViewModel(sessionManager) as T
        }
    }
}
