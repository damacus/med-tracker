package io.damacus.medtracker.ui.schedule

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
class ScheduleSessionTest {
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
        val model = ScheduleViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        val transitions: List<() -> Unit> = listOf(
            { signIn(2) }, { signIn(2, 2) }, { signIn(2, 2, "https://two.example/") },
            { sessions.clearSession() }, { signIn() }, { signIn() }
        )
        transitions.forEach { change ->
            if (sessions.sessionState.value.isLoggedIn) {
                model.createSchedule(CreateSchedulePayload(1, 1, 1.0, "mg", "daily", startDate = "2026-10-27")) {}
                runCurrent()
                assertTrue(model.uiState.value.successMessage != null)
            }
            change()
            assertTrue(model.uiState.value.schedules.isEmpty())
            assertNull(model.uiState.value.errorMessage)
            assertNull(model.uiState.value.successMessage)
            runCurrent()
        }
    }

    @Test fun `late load success cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateLoad("load", ApiResult.Success(listOf(ScheduleDto(id = 99))))
    }

    @Test fun `late load error cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateLoad("load", ApiResult.Error("old", "Old session error"))
    }

    @Test fun `late load network cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateLoad("load", ApiResult.NetworkError(IllegalStateException("Old network failure")))
    }

    @Test fun `late create success cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("create", ApiResult.Success(ScheduleDto(id = 99)))
    }

    @Test fun `late create error cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("create", ApiResult.Error("old", "Old session error"))
    }

    @Test fun `late create network cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("create", ApiResult.NetworkError(IllegalStateException("Old network failure")))
    }

    @Test fun `late pause success cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("pause", ApiResult.Success(ScheduleDto(id = 99)))
    }

    @Test fun `late pause error cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("pause", ApiResult.Error("old", "Old session error"))
    }

    @Test fun `late pause network cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("pause", ApiResult.NetworkError(IllegalStateException("Old network failure")))
    }

    @Test fun `late resume success cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("resume", ApiResult.Success(ScheduleDto(id = 99)))
    }

    @Test fun `late resume error cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("resume", ApiResult.Error("old", "Old session error"))
    }

    @Test fun `late resume network cannot change the next session or callback`() = runTest(dispatcher) {
        assertLateMutation("resume", ApiResult.NetworkError(IllegalStateException("Old network failure")))
    }

    private fun kotlinx.coroutines.test.TestScope.assertLateMutation(operation: String, result: ApiResult<ScheduleDto>) {
        signIn()
        val model = ScheduleViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        var callbacks = 0
        api.delayedOperation = operation
        when (operation) {
            "load" -> model.loadSchedules()
            "create" -> model.createSchedule(CreateSchedulePayload(1, 1, 1.0, "mg", "daily", startDate = "2026-10-27")) { callbacks++ }
            "pause" -> model.togglePauseSchedule(ScheduleDto(id = 1))
            "resume" -> model.togglePauseSchedule(ScheduleDto(id = 1, paused = true))
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

    private fun kotlinx.coroutines.test.TestScope.assertLateLoad(operation: String, result: ApiResult<List<ScheduleDto>>) {
        signIn()
        val model = ScheduleViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        api.delayedOperation = operation
        model.loadSchedules()
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
        val model = ScheduleViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        var callbacks = 0
        model.createSchedule(CreateSchedulePayload(1, 1, 1.0, "mg", "daily", startDate = "2026-10-27")) { callbacks++ }
        runCurrent()
        assertEquals(2, model.uiState.value.schedules.size)
        assertTrue(model.uiState.value.successMessage != null)
        assertEquals(1, callbacks)
    }

    @Test fun `render projection hides a collected old state when session collection advances first`() = runTest(dispatcher) {
        signIn()
        val model = ScheduleViewModel(sessions, api).also { store.put("model", it) }
        runCurrent()
        val collectedState = model.uiState.value.copy(
            isLoading = true,
            errorMessage = "Old error",
            successMessage = "Old success"
        )
        assertTrue(collectedState.schedules.isNotEmpty())
        assertEquals(collectedState, collectedState.forSession(sessions.sessionState.value))

        val transitions: List<() -> Unit> = listOf(
            { signIn(2) }, { signIn(2, 2) }, { signIn(2, 2, "https://two.example/") },
            { sessions.clearSession() }, { signIn() }
        )
        transitions.forEach { change ->
            change()
            val session = sessions.sessionState.value
            assertEquals(ScheduleUiState(sessionRevision = session.revision), collectedState.forSession(session))
            runCurrent()
            assertEquals(model.uiState.value, model.uiState.value.forSession(session))
        }
    }

    private class ControlledApi : MedTrackerApi {
        var delayedOperation: String? = null
        var pending: Continuation<ApiResult<ScheduleDto>>? = null
        var pendingLoad: Continuation<ApiResult<List<ScheduleDto>>>? = null
        var returned = false
        private suspend fun mutation(operation: String): ApiResult<ScheduleDto> {
            if (delayedOperation != operation) return ApiResult.Success(ScheduleDto(id = 1))
            val result = suspendCoroutine<ApiResult<ScheduleDto>> { pending = it }
            returned = true
            return result
        }
        override suspend fun logout(baseUrl: String, accessToken: String): ApiResult<Unit> = error("Not used")
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<PersonDto>> = error("Not used")
        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationDto>> = error("Not used")
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<ScheduleDto>> {
            if (delayedOperation != "load") return ApiResult.Success(listOf(ScheduleDto(id = 1)))
            val result = suspendCoroutine<ApiResult<List<ScheduleDto>>> { pendingLoad = it }
            returned = true
            return result
        }
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationTakeDto>> = error("Not used")
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload): ApiResult<MedicationTakeDto> = error("Not used")
        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<LocationDto>> = error("Not used")
        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HouseholdInvitationDto>> = error("Not used")
        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String): ApiResult<HouseholdInvitationDto> = error("Not used")
        override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload): ApiResult<MedicationDto> = error("Not used")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload): ApiResult<MedicationDto> = error("Not used")
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload): ApiResult<ScheduleDto> = mutation("create")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto> = mutation("pause")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto> = mutation("resume")
        override suspend fun getHealthEvents(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HealthEventDto>> = error("Not used")
        override suspend fun createHealthEvent(baseUrl: String, accessToken: String, householdId: Long, personId: Long, title: String, notes: String): ApiResult<HealthEventDto> = error("Not used")
    }
}
