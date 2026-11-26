package io.damacus.medtracker.ui.lookup

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.MedicationLookupResultDto
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class MedicationLookupUiState(
    val sessionRevision: String? = null,
    val query: String = "",
    val isLoading: Boolean = false,
    val results: List<MedicationLookupResultDto> = emptyList(),
    val errorMessage: String? = null
) {
    fun forSession(session: AppSession): MedicationLookupUiState =
        if (sessionRevision == session.revision) this else MedicationLookupUiState(sessionRevision = session.revision)
}

class MedicationLookupViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(MedicationLookupUiState())
    val uiState: StateFlow<MedicationLookupUiState> = _uiState.asStateFlow()
    private var sessionWork: Job = SupervisorJob(viewModelScope.coroutineContext[Job])
    private var searchJob: Job? = null
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            searchJob?.cancel()
            searchJob = null
            sessionWork.cancel()
            sessionWork = SupervisorJob(viewModelScope.coroutineContext[Job])
            _uiState.value = MedicationLookupUiState(sessionRevision = session.revision)
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        searchJob?.cancel()
        sessionWork.cancel()
        super.onCleared()
    }

    fun search(query: String) {
        val session = sessionManager.sessionState.value
        val work = sessionWork
        updateForSession(session, work) { it.copy(query = query) }
        if (query.length < 2) {
            searchJob?.cancel()
            updateForSession(session, work) { it.copy(results = emptyList(), isLoading = false) }
            return
        }

        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        searchJob?.cancel()
        searchJob = CoroutineScope(viewModelScope.coroutineContext + work).launch {
            delay(300) // Debounce typing
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(isLoading = true, errorMessage = null) }

            when (val res = apiClient.lookupMedication(session.serverUrl, token, householdId, query)) {
                is ApiResult.Success -> {
                    updateForSession(session, work) { it.copy(isLoading = false, results = res.data) }
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) { it.copy(isLoading = false, errorMessage = res.message) }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) { it.copy(isLoading = false, errorMessage = "Network error during lookup") }
                }
            }
        }
    }

    private fun isCurrentSession(session: AppSession, work: Job): Boolean =
        work.isActive && sessionManager.sessionState.value.revision == session.revision

    private fun updateForSession(session: AppSession, work: Job, update: (MedicationLookupUiState) -> MedicationLookupUiState) {
        _uiState.update { current ->
            if (isCurrentSession(session, work) && current.sessionRevision == session.revision) update(current) else current
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return MedicationLookupViewModel(sessionManager) as T
        }
    }
}
