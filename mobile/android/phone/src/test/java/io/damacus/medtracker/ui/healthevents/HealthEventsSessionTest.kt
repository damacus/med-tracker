package io.damacus.medtracker.ui.healthevents

import androidx.lifecycle.ViewModelStore
import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HealthEventDto
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.HouseholdInvitationDto
import io.damacus.medtracker.data.model.LocationDto
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.MedicationTakeDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

@OptIn(ExperimentalCoroutinesApi::class)
class HealthEventsSessionTest {
    private val dispatcher = StandardTestDispatcher()
    private val store = ViewModelStore()
    private val sessions = SessionManager(object : CredentialStore {
        private var saved: String? = null
        override fun read() = saved
        override fun write(value: String) { saved = value }
        override fun clear() { saved = null }
    })
    private val api = ControlledApi()

    @Before fun setUp() { Dispatchers.setMain(dispatcher) }
    @After fun tearDown() {
        store.clear()
        Dispatchers.resetMain()
    }

    private fun signIn(account: Long = 1, household: Long = 1, server: String = "https://one.example/") {
        sessions.saveSession(SessionPayload("token-$account", refreshToken = "refresh-$account", me = UserDto(id = account), household = HouseholdDto(id = household)), server)
    }

    @Test fun `session changes synchronously clear sensitive state`() = runTest(dispatcher) {
        signIn()
        val model = HealthEventsViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        val transitions: List<() -> Unit> = listOf(
            { signIn(2) }, { signIn(2, 2) }, { signIn(2, 2, "https://two.example/") },
            { sessions.clearSession() }, { signIn() }, { signIn() }
        )
        transitions.forEach { change ->
            if (sessions.sessionState.value.isLoggedIn) {
                model.createHealthEvent(1, "Title", "Notes") {}
                runCurrent()
                assertTrue(model.uiState.value.actionSuccessMessage != null)
            }
            change()
            assertTrue(model.uiState.value.events.isEmpty())
            assertNull(model.uiState.value.errorMessage)
            assertNull(model.uiState.value.actionSuccessMessage)
            runCurrent()
        }
    }

    @Test fun `late load success cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateLoad("load", ApiResult.Success(listOf(HealthEventDto(id = 99, eventKind = "suspected_side_effect", title = "Old event"))))
    }

    @Test fun `late load error cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateLoad("load", ApiResult.Error("old", "Old session error"))
    }

    @Test fun `late load network cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateLoad("load", ApiResult.NetworkError(IllegalStateException("Old network failure")))
    }

    @Test fun `late create success cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("create", ApiResult.Success(HealthEventDto(id = 99, eventKind = "suspected_side_effect", title = "Old event")))
    }

    @Test fun `late create error cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("create", ApiResult.Error("old", "Old session error"))
    }

    @Test fun `late create network cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("create", ApiResult.NetworkError(IllegalStateException("Old network failure")))
    }

    private fun kotlinx.coroutines.test.TestScope.assertLateMutation(operation: String, result: ApiResult<HealthEventDto>) {
        signIn()
        val model = HealthEventsViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        var callbacks = 0
        api.delayedOperation = operation
        when (operation) {
            "load" -> model.loadHealthEvents()
            "create" -> model.createHealthEvent(1, "Title", "Notes") { callbacks++ }
        }
        runCurrent()
        val pending = api.pending!!
        api.delayedOperation = null
        sessions.clearSession()
        signIn()
        runCurrent()
        val nextState = model.uiState.value
        pending.resume(result)
        runCurrent()
        assertTrue(api.returned)
        assertEquals(nextState, model.uiState.value)
        assertEquals(0, callbacks)
    }

    private fun kotlinx.coroutines.test.TestScope.assertLateLoad(operation: String, result: ApiResult<List<HealthEventDto>>) {
        signIn()
        val model = HealthEventsViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        api.delayedOperation = operation
        model.loadHealthEvents()
        runCurrent()
        val pending = api.pendingLoad!!
        api.delayedOperation = null
        sessions.clearSession()
        signIn()
        runCurrent()
        val nextState = model.uiState.value
        pending.resume(result)
        runCurrent()
        assertTrue(api.returned)
        assertEquals(nextState, model.uiState.value)
    }

    @Test fun `current session creation still updates state and calls back`() = runTest(dispatcher) {
        signIn()
        val model = HealthEventsViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        var callbacks = 0
        model.createHealthEvent(1, "Title", "Notes") { callbacks++ }
        runCurrent()
        assertEquals(2, model.uiState.value.events.size)
        assertTrue(model.uiState.value.actionSuccessMessage != null)
        assertEquals(1, callbacks)
    }

    @Test fun `render projection hides a collected old state when session collection advances first`() = runTest(dispatcher) {
        signIn()
        val model = HealthEventsViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        val collectedState = model.uiState.value.copy(
            isLoading = true,
            errorMessage = "Old error",
            actionSuccessMessage = "Old success"
        )
        assertTrue(collectedState.events.isNotEmpty())
        assertEquals(collectedState, collectedState.forSession(sessions.sessionState.value))

        val transitions: List<() -> Unit> = listOf(
            { signIn(2) }, { signIn(2, 2) }, { signIn(2, 2, "https://two.example/") },
            { sessions.clearSession() }, { signIn() }
        )
        transitions.forEach { change ->
            change()
            val session = sessions.sessionState.value
            assertEquals(HealthEventsUiState(sessionRevision = session.revision), collectedState.forSession(session))
            runCurrent()
            assertEquals(model.uiState.value, model.uiState.value.forSession(session))
        }
    }

    private class ControlledApi : MedTrackerApi {
        var delayedOperation: String? = null
        var pending: Continuation<ApiResult<HealthEventDto>>? = null
        var pendingLoad: Continuation<ApiResult<List<HealthEventDto>>>? = null
        var returned = false
        private suspend fun mutation(operation: String): ApiResult<HealthEventDto> {
            if (delayedOperation != operation) return ApiResult.Success(HealthEventDto(id = 1, eventKind = "suspected_side_effect", title = "Old event"))
            val result = suspendCoroutine<ApiResult<HealthEventDto>> { pending = it }
            returned = true
            return result
        }
        override suspend fun logout(baseUrl: String, accessToken: String): ApiResult<Unit> = error("Not used")
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<PersonDto>> = error("Not used")
        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationDto>> = error("Not used")
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<ScheduleDto>> = error("Not used")
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationTakeDto>> = error("Not used")
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload): ApiResult<MedicationTakeDto> = error("Not used")
        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<LocationDto>> = error("Not used")
        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HouseholdInvitationDto>> = error("Not used")
        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String): ApiResult<HouseholdInvitationDto> = error("Not used")
        override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload): ApiResult<MedicationDto> = error("Not used")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload): ApiResult<MedicationDto> = error("Not used")
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload): ApiResult<ScheduleDto> = error("Not used")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto> = error("Not used")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto> = error("Not used")
        override suspend fun getHealthEvents(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HealthEventDto>> {
            if (delayedOperation != "load") return ApiResult.Success(listOf(HealthEventDto(id = 1, eventKind = "suspected_side_effect", title = "Old event")))
            val result = suspendCoroutine<ApiResult<List<HealthEventDto>>> { pendingLoad = it }
            returned = true
            return result
        }
        override suspend fun createHealthEvent(baseUrl: String, accessToken: String, householdId: Long, personId: Long, title: String, notes: String): ApiResult<HealthEventDto> = mutation("create")
    }
}
