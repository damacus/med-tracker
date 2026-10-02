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
)

class HealthEventsViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(HealthEventsUiState())
    val uiState: StateFlow<HealthEventsUiState> = _uiState.asStateFlow()
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            _uiState.value = HealthEventsUiState(sessionRevision = session.revision)
            loadHealthEvents(session)
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        super.onCleared()
    }

    fun loadHealthEvents(session: AppSession = sessionManager.sessionState.value) {
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        _uiState.update { it.copy(isLoading = true, errorMessage = null) }

        viewModelScope.launch {
            when (val res = apiClient.getHealthEvents(session.serverUrl, token, householdId)) {
                is ApiResult.Success -> {
                    _uiState.update { it.copy(isLoading = false, events = res.data) }
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = res.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error loading health events") }
                }
            }
        }
    }

    fun createHealthEvent(personId: Long, title: String, notes: String, onSuccess: () -> Unit) {
        val session = sessionManager.sessionState.value
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        viewModelScope.launch {
            when (val res = apiClient.createHealthEvent(session.serverUrl, token, householdId, personId, title, notes)) {
                is ApiResult.Success -> {
                    _uiState.update { it.copy(events = listOf(res.data) + it.events, actionSuccessMessage = "Logged health event") }
                    onSuccess()
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(errorMessage = res.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(errorMessage = "Network error creating health event") }
                }
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return HealthEventsViewModel(sessionManager) as T
        }
    }
}
