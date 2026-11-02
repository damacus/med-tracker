package io.damacus.medtracker.ui.healthevents

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.HealthEventDto
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class HealthEventsUiState(
    val sessionRevision: String? = null,
    val isLoading: Boolean = false,
    val events: List<HealthEventDto> = emptyList(),
    val errorMessage: String? = null,
    val actionSuccessMessage: String? = null
) {
    fun forSession(session: AppSession): HealthEventsUiState =
        if (sessionRevision == session.revision) this else HealthEventsUiState(sessionRevision = session.revision)
}

class HealthEventsViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(HealthEventsUiState())
    val uiState: StateFlow<HealthEventsUiState> = _uiState.asStateFlow()
    private var sessionWork: Job = SupervisorJob(viewModelScope.coroutineContext[Job])
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            sessionWork.cancel()
            sessionWork = SupervisorJob(viewModelScope.coroutineContext[Job])
            _uiState.value = HealthEventsUiState(sessionRevision = session.revision)
            loadHealthEvents(session)
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        sessionWork.cancel()
        super.onCleared()
    }

    fun loadHealthEvents(session: AppSession = sessionManager.sessionState.value) {
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        val work = sessionWork
        updateForSession(session, work) { it.copy(isLoading = true, errorMessage = null) }

        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            when (val res = apiClient.getHealthEvents(session.serverUrl, token, householdId)) {
                is ApiResult.Success -> {
                    updateForSession(session, work) { it.copy(isLoading = false, events = res.data) }
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) { it.copy(isLoading = false, errorMessage = res.message) }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) { it.copy(isLoading = false, errorMessage = "Network error loading health events") }
                }
            }
        }
    }

    fun createHealthEvent(personId: Long, title: String, notes: String, onSuccess: () -> Unit) {
        val session = sessionManager.sessionState.value
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        val work = sessionWork
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            when (val res = apiClient.createHealthEvent(session.serverUrl, token, householdId, personId, title, notes)) {
                is ApiResult.Success -> {
                    updateForSession(session, work) { it.copy(events = listOf(res.data) + it.events, actionSuccessMessage = "Logged health event") }
                    if (isCurrentSession(session, work)) onSuccess()
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) { it.copy(errorMessage = res.message) }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) { it.copy(errorMessage = "Network error creating health event") }
                }
            }
        }
    }

    private fun isCurrentSession(session: AppSession, work: Job): Boolean =
        work.isActive && sessionManager.sessionState.value.revision == session.revision

    private fun updateForSession(session: AppSession, work: Job, update: (HealthEventsUiState) -> HealthEventsUiState) {
        _uiState.update { current ->
            if (isCurrentSession(session, work) && current.sessionRevision == session.revision) update(current) else current
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return HealthEventsViewModel(sessionManager) as T
        }
    }
}
