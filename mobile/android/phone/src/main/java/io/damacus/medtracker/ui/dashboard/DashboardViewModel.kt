package io.damacus.medtracker.ui.dashboard

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.DashboardData
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.MedicationTakeDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.ScheduleDto
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.format.DateTimeFormatter
import java.util.UUID

data class DashboardUiState(
    val isLoading: Boolean = false,
    val isRefreshing: Boolean = false,
    val takingScheduleId: Long? = null,
    val dashboardData: DashboardData = DashboardData(),
    val errorMessage: String? = null,
    val actionSuccessMessage: String? = null
)

class DashboardViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(DashboardUiState(isLoading = true))
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()

    init {
        loadDashboardData()
    }

    fun refresh() {
        loadDashboardData(isRefresh = true)
    }

    fun selectPerson(personId: Long?) {
        _uiState.update { current ->
            current.copy(
                dashboardData = current.dashboardData.copy(selectedPersonId = personId)
            )
        }
    }

    fun clearMessages() {
        _uiState.update { it.copy(errorMessage = null, actionSuccessMessage = null) }
    }

    fun recordDose(schedule: ScheduleDto) {
        val scheduleId = schedule.id ?: return
        val session = sessionManager.sessionState.value
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(takingScheduleId = scheduleId, errorMessage = null) }

            val nowIso = DateTimeFormatter.ISO_INSTANT.format(Instant.now())
            val clientUuid = UUID.randomUUID().toString()

            val request = RecordDosePayload(
                clientUuid = clientUuid,
                sourceType = "schedule",
                sourceId = scheduleId.toString(),
                takenAt = nowIso,
                doseAmount = schedule.doseAmount,
                doseUnit = schedule.doseUnit
            )

            val result = apiClient.recordDose(
                baseUrl = session.serverUrl,
                accessToken = token,
                householdId = householdId,
                request = request
            )

            when (result) {
                is ApiResult.Success -> {
                    _uiState.update { current ->
                        val updatedTakes = listOf(result.data) + current.dashboardData.recentTakes
                        current.copy(
                            takingScheduleId = null,
                            actionSuccessMessage = "Recorded dose successfully",
                            dashboardData = current.dashboardData.copy(recentTakes = updatedTakes)
                        )
                    }
                    // Refresh data in background to ensure stock and schedules are synced
                    loadDashboardData(isRefresh = true)
                }
                is ApiResult.Error -> {
                    _uiState.update {
                        it.copy(
                            takingScheduleId = null,
                            errorMessage = result.message
                        )
                    }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update {
                        it.copy(
                            takingScheduleId = null,
                            errorMessage = "Network error: ${result.cause.localizedMessage ?: "Unable to record dose"}"
                        )
                    }
                }
            }
        }
    }

    private fun loadDashboardData(isRefresh: Boolean = false) {
        val session = sessionManager.sessionState.value
        val householdId = session.household?.id
        val token = session.accessToken

        if (householdId == null || token.isNullOrBlank()) {
            _uiState.update {
                it.copy(
                    isLoading = false,
                    isRefreshing = false,
                    errorMessage = "No active household or session found"
                )
            }
            return
        }

        viewModelScope.launch {
            if (isRefresh) {
                _uiState.update { it.copy(isRefreshing = true, errorMessage = null) }
            } else {
                _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            }

            try {
                coroutineScope {
                    val peopleDeferred = async { apiClient.getPeople(session.serverUrl, token, householdId) }
                    val medsDeferred = async { apiClient.getMedications(session.serverUrl, token, householdId) }
                    val schedulesDeferred = async { apiClient.getSchedules(session.serverUrl, token, householdId) }
                    val takesDeferred = async { apiClient.getMedicationTakes(session.serverUrl, token, householdId) }

                    val peopleRes = peopleDeferred.await()
                    val medsRes = medsDeferred.await()
                    val schedulesRes = schedulesDeferred.await()
                    val takesRes = takesDeferred.await()

                    val errors = mutableListOf<String>()
                    val peopleList = when (peopleRes) {
                        is ApiResult.Success -> peopleRes.data
                        is ApiResult.Error -> { errors.add("People: ${peopleRes.message}"); emptyList() }
                        is ApiResult.NetworkError -> { errors.add("People: Network error"); emptyList() }
                    }

                    val medsList = when (medsRes) {
                        is ApiResult.Success -> medsRes.data
                        is ApiResult.Error -> { errors.add("Medications: ${medsRes.message}"); emptyList() }
                        is ApiResult.NetworkError -> { errors.add("Medications: Network error"); emptyList() }
                    }

                    val schedulesList = when (schedulesRes) {
                        is ApiResult.Success -> schedulesRes.data
                        is ApiResult.Error -> { errors.add("Schedules: ${schedulesRes.message}"); emptyList() }
                        is ApiResult.NetworkError -> { errors.add("Schedules: Network error"); emptyList() }
                    }

                    val takesList = when (takesRes) {
                        is ApiResult.Success -> takesRes.data
                        is ApiResult.Error -> { errors.add("Takes: ${takesRes.message}"); emptyList() }
                        is ApiResult.NetworkError -> { errors.add("Takes: Network error"); emptyList() }
                    }

                    _uiState.update { current ->
                        current.copy(
                            isLoading = false,
                            isRefreshing = false,
                            errorMessage = if (errors.isNotEmpty()) errors.joinToString("\n") else null,
                            dashboardData = current.dashboardData.copy(
                                people = peopleList,
                                medications = medsList,
                                schedules = schedulesList,
                                recentTakes = takesList
                            )
                        )
                    }
                }
            } catch (e: Exception) {
                _uiState.update {
                    it.copy(
                        isLoading = false,
                        isRefreshing = false,
                        errorMessage = "Error loading dashboard: ${e.localizedMessage}"
                    )
                }
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return DashboardViewModel(sessionManager) as T
        }
    }
}
