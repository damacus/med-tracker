package io.damacus.medtracker.ui.household

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.HouseholdInvitationDto
import io.damacus.medtracker.data.model.LocationDto
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class HouseholdUiState(
    val isLoading: Boolean = false,
    val locations: List<LocationDto> = emptyList(),
    val invitations: List<HouseholdInvitationDto> = emptyList(),
    val errorMessage: String? = null,
    val successMessage: String? = null
)

class HouseholdViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(HouseholdUiState())
    val uiState: StateFlow<HouseholdUiState> = _uiState.asStateFlow()
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            if (session.isLoggedIn) {
                loadData(session)
            } else {
                _uiState.value = HouseholdUiState()
            }
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        super.onCleared()
    }

    fun clearMessages() {
        _uiState.update { it.copy(errorMessage = null, successMessage = null) }
    }

    fun loadData(session: AppSession = sessionManager.sessionState.value) {
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            val locsResult = apiClient.getLocations(session.serverUrl, token, householdId)
            val invsResult = apiClient.getInvitations(session.serverUrl, token, householdId)

            val newLocs = if (locsResult is ApiResult.Success) locsResult.data else emptyList()
            val newInvs = if (invsResult is ApiResult.Success) invsResult.data else emptyList()

            _uiState.update {
                it.copy(
                    isLoading = false,
                    locations = newLocs,
                    invitations = newInvs
                )
            }
        }
    }

    fun createInvitation(email: String, role: String, onComplete: () -> Unit) {
        val session = sessionManager.sessionState.value
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            when (val result = apiClient.createInvitation(session.serverUrl, token, householdId, email, role)) {
                is ApiResult.Success -> {
                    _uiState.update { current ->
                        current.copy(
                            isLoading = false,
                            invitations = current.invitations + result.data,
                            successMessage = "Invitation sent to $email"
                        )
                    }
                    onComplete()
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error sending invitation") }
                }
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return HouseholdViewModel(sessionManager) as T
        }
    }
}
