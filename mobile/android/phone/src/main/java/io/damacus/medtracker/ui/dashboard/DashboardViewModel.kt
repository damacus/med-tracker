package io.damacus.medtracker.ui.dashboard

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
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
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
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
    val sessionRevision: String? = null,
    val isLoading: Boolean = false,
    val isRefreshing: Boolean = false,
    val takingScheduleId: Long? = null,
    val dashboardData: DashboardData = DashboardData(),
    val errorMessage: String? = null,
    val actionSuccessMessage: String? = null
) {
    fun forSession(session: AppSession): DashboardUiState =
        if (sessionRevision == session.revision) this else DashboardUiState(sessionRevision = session.revision)
}

class DashboardViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(DashboardUiState())
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()
    private var sessionWork: Job = SupervisorJob(viewModelScope.coroutineContext[Job])
    private var loadJob: Job? = null
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            sessionWork.cancel()
            sessionWork = SupervisorJob(viewModelScope.coroutineContext[Job])
            _uiState.value = DashboardUiState(sessionRevision = session.revision)
            loadDashboardData(session)
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        sessionWork.cancel()
        super.onCleared()
    }

    fun refresh(sessionRevision: String = sessionManager.sessionState.value.revision) {
        if (sessionRevision != sessionManager.sessionState.value.revision) return
        loadDashboardData(isRefresh = true)
    }

    fun selectPerson(personId: Long?, sessionRevision: String = sessionManager.sessionState.value.revision) {
        val session = sessionManager.sessionState.value
        if (sessionRevision != session.revision) return
        updateForSession(session, sessionWork) { current ->
            if (personId != null && current.dashboardData.people.none { it.id == personId }) return@updateForSession current
            current.copy(
                dashboardData = current.dashboardData.copy(selectedPersonId = personId)
            )
        }
    }

    fun clearMessages(sessionRevision: String = sessionManager.sessionState.value.revision) {
        val session = sessionManager.sessionState.value
        if (sessionRevision != session.revision) return
        updateForSession(session, sessionWork) { it.copy(errorMessage = null, actionSuccessMessage = null) }
    }

    fun recordDose(schedule: ScheduleDto, sessionRevision: String = sessionManager.sessionState.value.revision) {
        val scheduleId = schedule.id ?: return
        val session = sessionManager.sessionState.value
        if (sessionRevision != session.revision || _uiState.value.sessionRevision != session.revision) return
        if (schedule !in _uiState.value.dashboardData.schedules) return
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        val work = sessionWork
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(takingScheduleId = scheduleId, errorMessage = null) }

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
            if (!isCurrentSession(session, work)) return@launch

            when (result) {
                is ApiResult.Success -> {
                    updateForSession(session, work) { current ->
                        val updatedTakes = listOf(result.data) + current.dashboardData.recentTakes
                        current.copy(
                            takingScheduleId = null,
                            actionSuccessMessage = "Recorded dose successfully",
                            dashboardData = current.dashboardData.copy(recentTakes = updatedTakes)
                        )
                    }
                    // Refresh data in background to ensure stock and schedules are synced
                    loadDashboardData(session, isRefresh = true)
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) {
                        it.copy(
                            takingScheduleId = null,
                            errorMessage = result.message
                        )
                    }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) {
                        it.copy(
                            takingScheduleId = null,
                            errorMessage = "Network error: ${result.cause.localizedMessage ?: "Unable to record dose"}"
                        )
                    }
                }
            }
        }
    }

    private fun loadDashboardData(session: AppSession = sessionManager.sessionState.value, isRefresh: Boolean = false) {
        val work = sessionWork
        if (!isCurrentSession(session, work)) return
        val householdId = session.household?.id
        val token = session.accessToken

        if (householdId == null || token.isNullOrBlank()) {
            return
        }

        loadJob?.cancel()
        loadJob = CoroutineScope(viewModelScope.coroutineContext + work).launch {
            val requestJob = coroutineContext[Job]!!
            if (!isCurrentSession(session, requestJob)) return@launch
            if (isRefresh) {
                updateForSession(session, requestJob) { it.copy(isRefreshing = true, errorMessage = null) }
            } else {
                updateForSession(session, requestJob) { it.copy(isLoading = true, errorMessage = null) }
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

                    updateForSession(session, requestJob) { current ->
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
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                updateForSession(session, requestJob) {
                    it.copy(
                        isLoading = false,
                        isRefreshing = false,
                        errorMessage = "Error loading dashboard: ${e.localizedMessage}"
                    )
                }
            }
        }
    }

    private fun isCurrentSession(session: AppSession, work: Job): Boolean =
        work.isActive && sessionManager.sessionState.value.revision == session.revision

    private fun updateForSession(session: AppSession, work: Job, update: (DashboardUiState) -> DashboardUiState) {
        _uiState.update { current ->
            if (isCurrentSession(session, work) && current.sessionRevision == session.revision) update(current) else current
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return DashboardViewModel(sessionManager) as T
        }
    }
}
