package io.damacus.medtracker.ui.schedule

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.ScheduleDto
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ScheduleUiState(
    val isLoading: Boolean = false,
    val schedules: List<ScheduleDto> = emptyList(),
    val errorMessage: String? = null,
    val successMessage: String? = null
)

class ScheduleViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(ScheduleUiState())
    val uiState: StateFlow<ScheduleUiState> = _uiState.asStateFlow()
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            if (session.isLoggedIn) {
                loadSchedules(session)
            } else {
                _uiState.value = ScheduleUiState()
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

    fun loadSchedules(session: AppSession = sessionManager.sessionState.value) {
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            when (val result = apiClient.getSchedules(session.serverUrl, token, householdId)) {
                is ApiResult.Success -> {
                    _uiState.update { it.copy(isLoading = false, schedules = result.data) }
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error loading schedules") }
                }
            }
        }
    }

    fun createSchedule(payload: CreateSchedulePayload, onComplete: () -> Unit) {
        val session = sessionManager.sessionState.value
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            when (val result = apiClient.createSchedule(session.serverUrl, token, householdId, payload)) {
                is ApiResult.Success -> {
                    _uiState.update { current ->
                        current.copy(
                            isLoading = false,
                            schedules = current.schedules + result.data,
                            successMessage = "Schedule created"
                        )
                    }
                    onComplete()
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error creating schedule") }
                }
            }
        }
    }

    fun togglePauseSchedule(schedule: ScheduleDto) {
        val scheduleId = schedule.id ?: return
        val session = sessionManager.sessionState.value
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            val call = if (schedule.paused) {
                apiClient.resumeSchedule(session.serverUrl, token, householdId, scheduleId)
            } else {
                apiClient.pauseSchedule(session.serverUrl, token, householdId, scheduleId)
            }
            when (call) {
                is ApiResult.Success -> {
                    _uiState.update { current ->
                        current.copy(
                            isLoading = false,
                            schedules = current.schedules.map { if (it.id == scheduleId) call.data else it },
                            successMessage = if (call.data.paused) "Schedule paused" else "Schedule resumed"
                        )
                    }
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = call.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error updating schedule") }
                }
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return ScheduleViewModel(sessionManager) as T
        }
    }
}
