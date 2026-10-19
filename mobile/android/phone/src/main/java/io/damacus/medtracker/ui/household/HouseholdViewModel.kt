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
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class HouseholdUiState(
    val sessionRevision: String? = null,
    val isLoading: Boolean = false,
    val locations: List<LocationDto> = emptyList(),
    val invitations: List<HouseholdInvitationDto> = emptyList(),
    val errorMessage: String? = null,
    val successMessage: String? = null
) {
    fun forSession(session: AppSession): HouseholdUiState =
        if (sessionRevision == session.revision) this else HouseholdUiState(sessionRevision = session.revision)
}

class HouseholdViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(HouseholdUiState())
    val uiState: StateFlow<HouseholdUiState> = _uiState.asStateFlow()
    private var sessionWork: Job = SupervisorJob(viewModelScope.coroutineContext[Job])
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            sessionWork.cancel()
            sessionWork = SupervisorJob(viewModelScope.coroutineContext[Job])
            _uiState.value = HouseholdUiState(sessionRevision = session.revision)
            if (session.isLoggedIn) {
                loadData(session)
            }
        }
    }

    override fun onCleared() {
        sessionWork.cancel()
        sessionObservation.close()
        super.onCleared()
    }

    fun clearMessages() {
        _uiState.update { it.copy(errorMessage = null, successMessage = null) }
    }

    fun loadData(session: AppSession = sessionManager.sessionState.value) {
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        val work = sessionWork
        if (!isCurrentSession(session, work)) return
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(isLoading = true) }
            val locsResult = apiClient.getLocations(session.serverUrl, token, householdId)
            if (!isCurrentSession(session, work)) return@launch
            val invsResult = apiClient.getInvitations(session.serverUrl, token, householdId)

            val newLocs = if (locsResult is ApiResult.Success) locsResult.data else emptyList()
            val newInvs = if (invsResult is ApiResult.Success) invsResult.data else emptyList()

            updateForSession(session, work) {
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

        val work = sessionWork
        if (!isCurrentSession(session, work)) return
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(isLoading = true) }
            when (val result = apiClient.createInvitation(session.serverUrl, token, householdId, email, role)) {
                is ApiResult.Success -> {
                    updateForSession(session, work) { current ->
                        current.copy(
                            isLoading = false,
                            invitations = current.invitations + result.data,
                            successMessage = "Invitation sent to $email"
                        )
                    }
                    if (isCurrentSession(session, work)) onComplete()
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) { it.copy(isLoading = false, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) { it.copy(isLoading = false, errorMessage = "Network error sending invitation") }
                }
            }
        }
    }

    private fun isCurrentSession(session: AppSession, work: Job): Boolean =
        work.isActive && sessionManager.sessionState.value.revision == session.revision

    private fun updateForSession(session: AppSession, work: Job, update: (HouseholdUiState) -> HouseholdUiState) {
        _uiState.update { current ->
            if (isCurrentSession(session, work) && current.sessionRevision == session.revision) update(current) else current
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return HouseholdViewModel(sessionManager) as T
        }
    }
}
