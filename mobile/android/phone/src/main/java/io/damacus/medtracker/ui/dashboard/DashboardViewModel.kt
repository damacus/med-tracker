package io.damacus.medtracker.ui.dashboard

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.DoseOccurrenceDto
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.GeneratedMedicationPauseGateway
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.api.MedicationPauseGateway
import io.damacus.medtracker.data.model.DashboardData
import io.damacus.medtracker.data.model.DashboardScheduleItem
import io.damacus.medtracker.data.model.MedicationTakeDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.data.offline.MutationType
import io.damacus.medtracker.data.offline.OfflineMutation
import io.damacus.medtracker.data.offline.OfflineQueueRepository
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
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.time.Instant
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.UUID

@Serializable
private data class QueuedDoseMutation(
    val clientUuid: String,
    val scheduleId: Long,
    val takenAt: String,
    val doseAmount: Double? = null,
    val doseUnit: String? = null,
    val occurrenceKey: String? = null,
    val etag: String? = null
)

private val offlineJson = Json { ignoreUnknownKeys = true }

data class DashboardUiState(
    val sessionRevision: String? = null,
    val isLoading: Boolean = false,
    val isRefreshing: Boolean = false,
    val takingScheduleId: Long? = null,
    val dashboardData: DashboardData = DashboardData(),
    val doseOccurrencesByScheduleId: Map<Long, DoseOccurrenceDto> = emptyMap(),
    val upcomingDoseOccurrences: List<DoseOccurrenceDto> = emptyList(),
    val failedOccurrenceScheduleIds: Set<Long>? = null,
    val isLoadingAiSuggestions: Boolean = false,
    val aiSuggestionsRequested: Boolean = false,
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
    private val pauseGateway: MedicationPauseGateway? = if (apiClient is GeneratedMedTrackerApi) GeneratedMedicationPauseGateway(apiClient) else null,
    private val offlineQueueRepository: OfflineQueueRepository? = null
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

    fun requestAiSuggestions(sessionRevision: String = sessionManager.sessionState.value.revision) {
        val session = sessionManager.sessionState.value
        if (sessionRevision != session.revision || _uiState.value.sessionRevision != session.revision) return
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        val work = sessionWork
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) {
                it.copy(isLoadingAiSuggestions = true, aiSuggestionsRequested = true, errorMessage = null)
            }
            when (val result = apiClient.getAiMedicationSuggestions(session.serverUrl, token, householdId)) {
                is ApiResult.Success -> {
                    val suggestions = result.data?.let { listOf(it) } ?: emptyList()
                    updateForSession(session, work) { current ->
                        current.copy(
                            isLoadingAiSuggestions = false,
                            dashboardData = current.dashboardData.copy(aiSuggestions = suggestions)
                        )
                    }
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) {
                        it.copy(isLoadingAiSuggestions = false, errorMessage = result.message)
                    }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) {
                        it.copy(isLoadingAiSuggestions = false, errorMessage = "Network error loading smart insights")
                    }
                }
            }
        }
    }

    fun recordDose(schedule: ScheduleDto, sessionRevision: String = sessionManager.sessionState.value.revision) {
        val scheduleId = schedule.id ?: return
        val session = sessionManager.sessionState.value
        if (sessionRevision != session.revision || _uiState.value.sessionRevision != session.revision) return
        if (schedule !in _uiState.value.dashboardData.schedules) return
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        val occurrence = _uiState.value.doseOccurrencesByScheduleId[scheduleId]
        val work = sessionWork
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(takingScheduleId = scheduleId, errorMessage = null) }

            val nowIso = DateTimeFormatter.ISO_INSTANT.format(Instant.now())
            val clientUuid = UUID.randomUUID().toString()

            if (occurrence != null) {
                val result = apiClient.takeScheduleDoseOccurrence(
                    baseUrl = session.serverUrl,
                    accessToken = token,
                    householdId = householdId,
                    scheduleId = scheduleId,
                    occurrenceKey = occurrence.key,
                    takenAt = nowIso,
                    clientUuid = clientUuid,
                    doseAmount = schedule.doseAmount,
                    etag = occurrence.etag
                )
                if (!isCurrentSession(session, work)) return@launch
                when (result) {
                    is ApiResult.Success -> {
                        val takeRecord = MedicationTakeDto(
                            clientUuid = clientUuid,
                            scheduleId = scheduleId,
                            personId = schedule.personId,
                            medicationId = schedule.medicationId,
                            doseAmount = schedule.doseAmount,
                            doseUnit = schedule.doseUnit,
                            takenAt = nowIso
                        )
                        updateForSession(session, work) { current ->
                            current.copy(
                                takingScheduleId = null,
                                actionSuccessMessage = "Recorded dose successfully",
                                dashboardData = current.dashboardData.copy(
                                    recentTakes = listOf(takeRecord) + current.dashboardData.recentTakes
                                ),
                                doseOccurrencesByScheduleId = current.doseOccurrencesByScheduleId + (scheduleId to result.data),
                                upcomingDoseOccurrences = current.upcomingDoseOccurrences.map {
                                    if (it.key == result.data.key) result.data else it
                                }
                            )
                        }
                    }
                    is ApiResult.Error -> {
                        updateForSession(session, work) {
                            it.copy(takingScheduleId = null, errorMessage = result.message)
                        }
                    }
                    is ApiResult.NetworkError -> {
                        enqueueOfflineDoseOrError(
                            session = session,
                            work = work,
                            householdId = householdId,
                            queued = QueuedDoseMutation(
                                clientUuid = clientUuid,
                                scheduleId = scheduleId,
                                takenAt = nowIso,
                                doseAmount = schedule.doseAmount,
                                doseUnit = schedule.doseUnit,
                                occurrenceKey = occurrence.key,
                                etag = occurrence.etag
                            ),
                            cause = result.cause
                        )
                    }
                }
            } else {
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
                    }
                    is ApiResult.Error -> {
                        updateForSession(session, work) {
                            it.copy(takingScheduleId = null, errorMessage = result.message)
                        }
                    }
                    is ApiResult.NetworkError -> {
                        enqueueOfflineDoseOrError(
                            session = session,
                            work = work,
                            householdId = householdId,
                            queued = QueuedDoseMutation(
                                clientUuid = clientUuid,
                                scheduleId = scheduleId,
                                takenAt = nowIso,
                                doseAmount = schedule.doseAmount,
                                doseUnit = schedule.doseUnit
                            ),
                            cause = result.cause
                        )
                    }
                }
            }
        }
    }

    private suspend fun enqueueOfflineDoseOrError(
        session: AppSession,
        work: Job,
        householdId: Long,
        queued: QueuedDoseMutation,
        cause: Throwable
    ) {
        val queue = offlineQueueRepository
        val owner = session.owner
        if (queue != null && owner != null) {
            queue.enqueue(
                OfflineMutation(
                    id = queued.clientUuid,
                    householdId = householdId,
                    owner = owner,
                    type = MutationType.RECORD_DOSE,
                    payloadJson = offlineJson.encodeToString(queued)
                )
            )
            updateForSession(session, work) {
                it.copy(
                    takingScheduleId = null,
                    actionSuccessMessage = "Saved dose offline; will sync when reconnected"
                )
            }
        } else {
            updateForSession(session, work) {
                it.copy(
                    takingScheduleId = null,
                    errorMessage = "Network error: ${cause.localizedMessage ?: "Unable to record dose"}"
                )
            }
        }
    }

    fun flushOfflineQueue(sessionRevision: String = sessionManager.sessionState.value.revision) {
        val queue = offlineQueueRepository ?: return
        val session = sessionManager.sessionState.value
        val owner = session.owner ?: return
        if (sessionRevision != session.revision || _uiState.value.sessionRevision != session.revision) return
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        val work = sessionWork
        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            val flushResult = queue.flushPending(eligible = {
                it.owner == owner && it.householdId == householdId && it.type == MutationType.RECORD_DOSE
            }) { mutation ->
                if (!isCurrentSession(session, work)) {
                    return@flushPending Result.failure(CancellationException("Session changed"))
                }
                val queued = offlineJson.decodeFromString<QueuedDoseMutation>(mutation.payloadJson)
                val apiResult = if (queued.occurrenceKey != null) {
                    apiClient.takeScheduleDoseOccurrence(
                        baseUrl = session.serverUrl,
                        accessToken = token,
                        householdId = householdId,
                        scheduleId = queued.scheduleId,
                        occurrenceKey = queued.occurrenceKey,
                        takenAt = queued.takenAt,
                        clientUuid = queued.clientUuid,
                        doseAmount = queued.doseAmount,
                        etag = queued.etag
                    )
                } else {
                    apiClient.recordDose(
                        baseUrl = session.serverUrl,
                        accessToken = token,
                        householdId = householdId,
                        request = RecordDosePayload(
                            clientUuid = queued.clientUuid,
                            sourceType = "schedule",
                            sourceId = queued.scheduleId.toString(),
                            takenAt = queued.takenAt,
                            doseAmount = queued.doseAmount,
                            doseUnit = queued.doseUnit
                        )
                    )
                }
                when (apiResult) {
                    is ApiResult.Success -> Result.success(Unit)
                    is ApiResult.Error -> Result.failure(IllegalStateException(apiResult.message))
                    is ApiResult.NetworkError -> Result.failure(apiResult.cause)
                }
            }
            if (!isCurrentSession(session, work)) return@launch
            if (flushResult.succeeded > 0) {
                updateForSession(session, work) {
                    it.copy(actionSuccessMessage = "Synced ${flushResult.succeeded} offline dose(s)")
                }
                loadDashboardData(session, isRefresh = true)
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
                            doseOccurrencesByScheduleId = current.doseOccurrencesByScheduleId + (scheduleId to result.data),
                            upcomingDoseOccurrences = current.upcomingDoseOccurrences.map {
                                if (it.key == result.data.key) result.data else it
                            }
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
                            doseOccurrencesByScheduleId = current.doseOccurrencesByScheduleId + (scheduleId to result.data),
                            upcomingDoseOccurrences = current.upcomingDoseOccurrences.map {
                                if (it.key == result.data.key) result.data else it
                            }
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
                    val todayDate = LocalDate.now()
                    val today = todayDate.toString()
                    val windowEnd = todayDate.plusDays(7).toString()
                    val perScheduleOccurrences = schedulesList
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
                                    windowEnd
                                )
                                val rows = when (res) {
                                    is ApiResult.Success -> res.data
                                    is ApiResult.NetworkError -> null
                                    is ApiResult.Error -> if (res.statusCode in setOf(401, 403, 404)) emptyList() else null
                                }
                                scheduleId to rows
                            }
                        }
                        .awaitAll()

                    val allUpcoming = perScheduleOccurrences.flatMap { it.second.orEmpty() }
                    val failedIds = if (schedulesRes is ApiResult.Success) {
                        perScheduleOccurrences.filter { it.second == null }.map { it.first }.toSet()
                    } else if (schedulesRes is ApiResult.Error && schedulesRes.statusCode in setOf(401, 403, 404)) {
                        emptySet()
                    } else null
                    val occurrencesByScheduleId = perScheduleOccurrences.mapNotNull { (scheduleId, rows) ->
                        val list = rows ?: return@mapNotNull null
                        val todays = list.filter { occ ->
                            occ.windowStartsOn == null || occ.windowStartsOn == today
                        }
                        val pool = todays.ifEmpty { list }
                        val selected = pool.firstOrNull { it.outcome == "open" && it.due }
                            ?: pool.firstOrNull { it.outcome == "open" }
                            ?: pool.firstOrNull()
                        selected?.let { scheduleId to it }
                    }.toMap()

                    updateForSession(session, requestJob) { current ->
                        current.copy(
                            isLoading = false,
                            isRefreshing = false,
                            errorMessage = if (errors.isNotEmpty()) errors.joinToString("\n") else null,
                            doseOccurrencesByScheduleId = occurrencesByScheduleId,
                            upcomingDoseOccurrences = allUpcoming,
                            failedOccurrenceScheduleIds = failedIds,
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

    class Factory(
        private val sessionManager: SessionManager,
        private val offlineQueueRepository: OfflineQueueRepository? = null
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return DashboardViewModel(sessionManager, offlineQueueRepository = offlineQueueRepository) as T
        }
    }
}
