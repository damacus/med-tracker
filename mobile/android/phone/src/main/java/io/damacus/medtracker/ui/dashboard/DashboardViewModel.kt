package io.damacus.medtracker.ui.dashboard

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.DoseOccurrenceDto
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.api.MedicationPauseGateway
import io.damacus.medtracker.data.api.GeneratedMedicationPauseGateway
import io.damacus.medtracker.data.model.DashboardData
import io.damacus.medtracker.data.model.DashboardScheduleItem
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
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.UUID

data class DashboardUiState(
    val sessionRevision: String? = null,
    val isLoading: Boolean = false,
    val isRefreshing: Boolean = false,
    val takingScheduleId: Long? = null,
    val dashboardData: DashboardData = DashboardData(),
    val doseOccurrencesByScheduleId: Map<Long, DoseOccurrenceDto> = emptyMap(),
    val errorMessage: String? = null,
    val actionSuccessMessage: String? = null,
    val pauseController: MedicationPauseController? = null
) {
    fun forSession(session: AppSession): DashboardUiState =
        if (sessionRevision == session.revision) this else DashboardUiState(sessionRevision = session.revision)

    val displayScheduleItems: List<DashboardScheduleItem>
        get() {
            val today = LocalDate.now().toString()
            return dashboardData.displayScheduleItems.map { item ->
                val occurrence = item.schedule.id?.let { doseOccurrencesByScheduleId[it] }
                val isDueNow = if (occurrence != null) {
                    occurrence.outcome == "open" && occurrence.due
                } else {
                    val takenToday = item.lastTakenAt?.startsWith(today) == true
                    item.schedule.active && !item.schedule.paused && !takenToday
                }
                item.copy(isDueNow = isDueNow)
            }
        }
}

class DashboardViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi(),
    private val pauseGateway: MedicationPauseGateway? = if (apiClient is GeneratedMedTrackerApi) GeneratedMedicationPauseGateway(apiClient) else null
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
            if (session.isLoggedIn && session.household != null) {
                if (pauseGateway != null) {
                    val work = sessionWork
                    val controller = MedicationPauseController(session, pauseGateway, CoroutineScope(viewModelScope.coroutineContext + work),
                        { isCurrentSession(session, work) }, { loadDashboardData(session, isRefresh = true) })
                    _uiState.update { it.copy(pauseController = controller) }
                    controller.refresh()
                }
                loadDashboardData(session)
            }
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
        _uiState.value.pauseController?.refresh()
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
                        val existingOccurrence = current.doseOccurrencesByScheduleId[scheduleId]
                        val updatedOccurrences = if (existingOccurrence != null) {
                            current.doseOccurrencesByScheduleId + (scheduleId to existingOccurrence.copy(outcome = "taken", due = false))
                        } else {
                            current.doseOccurrencesByScheduleId
                        }
                        current.copy(
                            takingScheduleId = null,
                            actionSuccessMessage = "Recorded dose successfully",
                            dashboardData = current.dashboardData.copy(recentTakes = updatedTakes),
                            doseOccurrencesByScheduleId = updatedOccurrences
                        )
                    }
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

    fun recordDoseNotTaken(
        schedule: ScheduleDto,
        reason: String,
        notes: String,
        sessionRevision: String = sessionManager.sessionState.value.revision
    ) {
        val scheduleId = schedule.id ?: return
        val session = sessionManager.sessionState.value
        if (sessionRevision != session.revision || _uiState.value.sessionRevision != session.revision) return
        if (schedule !in _uiState.value.dashboardData.schedules) return
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return
        val occurrence = _uiState.value.doseOccurrencesByScheduleId[scheduleId]
        if (occurrence == null) {
            updateForSession(session, sessionWork) {
                it.copy(errorMessage = "No scheduled dose occurrence available to mark as not taken.")
            }
            return
        }

        val work = sessionWork
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(takingScheduleId = scheduleId, errorMessage = null) }
            val result = apiClient.recordScheduleDoseNotTaken(
                baseUrl = session.serverUrl,
                accessToken = token,
                householdId = householdId,
                scheduleId = scheduleId,
                occurrenceKey = occurrence.key,
                reason = reason,
                note = notes
            )
            if (!isCurrentSession(session, work)) return@launch
            when (result) {
                is ApiResult.Success -> {
                    updateForSession(session, work) { current ->
                        current.copy(
                            takingScheduleId = null,
                            actionSuccessMessage = "Recorded dose as not taken",
                            doseOccurrencesByScheduleId = current.doseOccurrencesByScheduleId + (scheduleId to result.data)
                        )
                    }
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) { it.copy(takingScheduleId = null, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) {
                        it.copy(takingScheduleId = null, errorMessage = "Network error: ${result.cause.localizedMessage ?: "Unable to record outcome"}")
                    }
                }
            }
        }
    }

    fun reopenDose(
        schedule: ScheduleDto,
        sessionRevision: String = sessionManager.sessionState.value.revision
    ) {
        val scheduleId = schedule.id ?: return
        val session = sessionManager.sessionState.value
        if (sessionRevision != session.revision || _uiState.value.sessionRevision != session.revision) return
        if (schedule !in _uiState.value.dashboardData.schedules) return
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return
        val occurrence = _uiState.value.doseOccurrencesByScheduleId[scheduleId]
        val etag = occurrence?.etag
        if (occurrence == null || etag.isNullOrBlank()) {
            updateForSession(session, sessionWork) {
                it.copy(errorMessage = "Unable to reopen dose without a recorded outcome.")
            }
            return
        }

        val work = sessionWork
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(takingScheduleId = scheduleId, errorMessage = null) }
            val result = apiClient.reopenScheduleDoseOccurrence(
                baseUrl = session.serverUrl,
                accessToken = token,
                householdId = householdId,
                scheduleId = scheduleId,
                occurrenceKey = occurrence.key,
                etag = etag
            )
            if (!isCurrentSession(session, work)) return@launch
            when (result) {
                is ApiResult.Success -> {
                    updateForSession(session, work) { current ->
                        current.copy(
                            takingScheduleId = null,
                            actionSuccessMessage = "Reopened dose occurrence",
                            doseOccurrencesByScheduleId = current.doseOccurrencesByScheduleId + (scheduleId to result.data)
                        )
                    }
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) { it.copy(takingScheduleId = null, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) {
                        it.copy(takingScheduleId = null, errorMessage = "Network error: ${result.cause.localizedMessage ?: "Unable to reopen dose"}")
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
                    val capabilitiesDeferred = async { apiClient.getCapabilities(session.serverUrl) }

                    val peopleRes = peopleDeferred.await()
                    val medsRes = medsDeferred.await()
                    val schedulesRes = schedulesDeferred.await()
                    val takesRes = takesDeferred.await()
                    val capabilitiesRes = capabilitiesDeferred.await()

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

                    val capabilities = (capabilitiesRes as? ApiResult.Success)?.data
                    val today = LocalDate.now().toString()
                    val occurrencesByScheduleId = schedulesList
                        .filter { it.active && !it.paused && it.id != null }
                        .map { schedule ->
                            val scheduleId = schedule.id!!
                            async {
                                val res = apiClient.getScheduleDoseOccurrences(
                                    session.serverUrl,
                                    token,
                                    householdId,
                                    scheduleId,
                                    today,
                                    today
                                )
                                val list = (res as? ApiResult.Success)?.data.orEmpty()
                                val selected = list.firstOrNull { it.outcome == "open" && it.due }
                                    ?: list.firstOrNull { it.outcome == "open" }
                                    ?: list.firstOrNull()
                                selected?.let { scheduleId to it }
                            }
                        }
                        .awaitAll()
                        .filterNotNull()
                        .toMap()

                    updateForSession(session, requestJob) { current ->
                        current.copy(
                            isLoading = false,
                            isRefreshing = false,
                            errorMessage = if (errors.isNotEmpty()) errors.joinToString("\n") else null,
                            doseOccurrencesByScheduleId = occurrencesByScheduleId,
                            dashboardData = current.dashboardData.copy(
                                people = peopleList,
                                medications = medsList,
                                schedules = schedulesList,
                                recentTakes = takesList,
                                capabilities = capabilities
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
