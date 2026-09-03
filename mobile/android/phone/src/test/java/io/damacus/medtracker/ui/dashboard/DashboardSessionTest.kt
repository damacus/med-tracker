package io.damacus.medtracker.ui.dashboard

import androidx.lifecycle.ViewModelStore
import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.*
import io.damacus.medtracker.ui.MainViewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

@OptIn(ExperimentalCoroutinesApi::class)
class DashboardSessionTest {
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

    @Test fun `signed out launch automatically loads after sign in`() = runTest(dispatcher) {
        val model = model()
        runCurrent()
        assertEquals(DashboardData(), model.uiState.value.dashboardData)
        assertTrue(api.peopleRequests.isEmpty())

        signIn(1)
        runCurrent()

        assertEquals(listOf(1L), model.uiState.value.dashboardData.people.map { it.id })
        assertFalse(model.uiState.value.isLoading)
        assertNull(model.uiState.value.errorMessage)
    }

    @Test fun `logout clears data selection and messages before another account signs in`() = runTest(dispatcher) {
        signIn(1)
        val model = model()
        runCurrent()
        model.selectPerson(1)
        model.recordDose(model.uiState.value.dashboardData.schedules.single())
        runCurrent()
        assertNotNull(model.uiState.value.actionSuccessMessage)

        val main = MainViewModel(sessions, api)
        store.put("main", main)
        main.logout()
        runCurrent()

        assertCleared(model.uiState.value)
        signIn(2)
        assertCleared(model.uiState.value)
        runCurrent()
        assertEquals(listOf(2L), model.uiState.value.dashboardData.people.map { it.id })
        assertNull(model.uiState.value.dashboardData.selectedPerson)
        assertNull(model.uiState.value.dashboardData.selectedPersonId)
    }

    @Test fun `account household server and new login each synchronously reset the session`() = runTest(dispatcher) {
        signIn(1)
        val model = model()
        runCurrent()
        val identities = listOf(
            Triple(2L, 1L, "https://one.example/"),
            Triple(2L, 2L, "https://one.example/"),
            Triple(2L, 2L, "https://two.example/"),
            Triple(2L, 2L, "https://two.example/")
        )
        identities.forEach { (account, household, server) ->
            model.selectPerson(model.uiState.value.dashboardData.people.single().id)
            signIn(account, household, server)
            assertCleared(model.uiState.value)
            runCurrent()
            assertEquals(account, model.uiState.value.dashboardData.people.single().id)
            assertNull(model.uiState.value.dashboardData.selectedPersonId)
            assertEquals(server to household, api.peopleRequests.last())
        }
    }

    @Test fun `late cancellation ignoring load cannot repopulate the next session`() = runTest(dispatcher) {
        signIn(1)
        api.delayPeople = true
        val model = model()
        runCurrent()
        val oldLoad = api.peopleJob!!
        signIn(2)
        assertTrue(oldLoad.isCancelled)
        runCurrent()
        val nextState = model.uiState.value

        api.peopleContinuation!!.resume(ApiResult.Success(listOf(PersonDto(id = 1, name = "Old person"))))
        runCurrent()

        assertTrue(api.oldPeopleReturned)
        assertEquals(nextState, model.uiState.value)
        assertEquals(2L, model.uiState.value.dashboardData.people.single().id)
    }

    @Test fun `late cancellation ignoring dose success cannot alter the next session`() = runTest(dispatcher) {
        assertLateDoseIgnored(ApiResult.Success(MedicationTakeDto(id = 100, personId = 1)))
    }

    @Test fun `late cancellation ignoring dose error cannot alter the next session`() = runTest(dispatcher) {
        assertLateDoseIgnored(ApiResult.Error("old", "Old session error"))
    }

    @Test fun `late cancellation ignoring dose network error cannot alter the next session`() = runTest(dispatcher) {
        assertLateDoseIgnored(ApiResult.NetworkError(IllegalStateException("Old network failure")))
    }

    @Test fun `render projection hides prior dashboard and profile before observers catch up`() = runTest(dispatcher) {
        signIn(1)
        val model = model()
        runCurrent()
        model.selectPerson(1)
        val previouslyCollectedState = model.uiState.value
        assertNotNull(previouslyCollectedState.dashboardData.selectedPerson)

        sessions.clearSession()
        assertCleared(previouslyCollectedState.forSession(sessions.sessionState.value))
        signIn(2)
        assertCleared(previouslyCollectedState.forSession(sessions.sessionState.value))
        runCurrent()
        assertEquals(model.uiState.value, model.uiState.value.forSession(sessions.sessionState.value))
    }

    @Test fun `cleared view model cancels requests and releases its session observer`() = runTest(dispatcher) {
        signIn(1)
        val model = model()
        api.delayPeople = true
        runCurrent()
        val oldLoad = api.peopleJob!!
        store.clear()
        val clearedState = model.uiState.value
        signIn(2)
        api.peopleContinuation!!.resume(ApiResult.Success(emptyList()))
        runCurrent()
        assertTrue(oldLoad.isCancelled)
        assertEquals(clearedState, model.uiState.value)
        assertEquals(1, api.peopleRequests.size)
    }

    @Test fun `rapid sign out and identical sign in invalidate the old dose result`() = runTest(dispatcher) {
        signIn(1)
        val model = model()
        runCurrent()
        val oldRevision = sessions.sessionState.value.revision
        api.delayDose = true
        model.recordDose(model.uiState.value.dashboardData.schedules.single())
        runCurrent()

        sessions.clearSession()
        assertCleared(model.uiState.value)
        signIn(1)
        assertNotEquals(oldRevision, sessions.sessionState.value.revision)
        assertCleared(model.uiState.value)
        runCurrent()
        val nextState = model.uiState.value
        api.doseContinuation!!.resume(ApiResult.Success(MedicationTakeDto(id = 100)))
        runCurrent()

        assertTrue(api.oldDoseReturned)
        assertEquals(nextState, model.uiState.value)
        assertEquals(2, api.peopleRequests.size)
    }

    @Test fun `callbacks from an old screen cannot change or submit the new session`() = runTest(dispatcher) {
        signIn(1)
        val model = model()
        runCurrent()
        val oldRevision = sessions.sessionState.value.revision
        signIn(1, household = 2)
        runCurrent()
        val schedule = model.uiState.value.dashboardData.schedules.single()
        model.recordDose(schedule)
        runCurrent()
        val nextState = model.uiState.value
        val requests = api.peopleRequests.size
        val doses = api.doseRequests

        model.selectPerson(1, oldRevision)
        model.clearMessages(oldRevision)
        model.refresh(oldRevision)
        model.recordDose(schedule, oldRevision)
        runCurrent()

        assertEquals(nextState, model.uiState.value)
        assertEquals(requests, api.peopleRequests.size)
        assertEquals(doses, api.doseRequests)
    }

    @Test fun `cancelled refresh cannot clear a later session with a load error`() = runTest(dispatcher) {
        signIn(1)
        val model = model()
        runCurrent()
        api.delayPeople = true
        model.refresh()
        runCurrent()
        assertTrue(model.uiState.value.isRefreshing)
        signIn(2)
        runCurrent()
        val nextState = model.uiState.value
        api.peopleContinuation!!.resumeWith(Result.failure(IllegalStateException("Old load failure")))
        runCurrent()

        assertEquals(nextState, model.uiState.value)
        assertNull(model.uiState.value.errorMessage)
    }

    @Test fun `logout clears locally before remote revoke and late completion cannot sign out the next account`() = runTest(dispatcher) {
        assertLateLogoutIgnored(fail = false)
    }

    @Test fun `late logout failure cannot clear or change the next accounts auth screen`() = runTest(dispatcher) {
        assertLateLogoutIgnored(fail = true)
    }

    @Test fun `logout callback from an old screen cannot sign out the current session`() = runTest(dispatcher) {
        signIn(1)
        val oldRevision = sessions.sessionState.value.revision
        val main = MainViewModel(sessions, api)
        store.put("main", main)
        signIn(2)
        val nextSession = sessions.sessionState.value

        main.logout(oldRevision)
        runCurrent()

        assertEquals(nextSession, sessions.sessionState.value)
        assertTrue(api.logoutTokens.isEmpty())
    }

    private suspend fun kotlinx.coroutines.test.TestScope.assertLateLogoutIgnored(fail: Boolean) {
        signIn(1)
        val model = model()
        val main = MainViewModel(sessions, api)
        store.put("main", main)
        runCurrent()
        api.delayLogout = true
        main.logout()
        runCurrent()
        assertFalse(sessions.sessionState.value.isLoggedIn)
        assertCleared(model.uiState.value)

        signIn(2)
        runCurrent()
        val nextSession = sessions.sessionState.value
        val nextDashboard = model.uiState.value
        val nextAuth = main.uiState.value
        if (fail) {
            api.logoutContinuation!!.resumeWith(Result.failure(IllegalStateException("Old revoke failed")))
        } else {
            api.logoutContinuation!!.resume(ApiResult.Success(Unit))
        }
        runCurrent()

        assertEquals(nextSession, sessions.sessionState.value)
        assertEquals(nextDashboard, model.uiState.value)
        assertEquals(nextAuth, main.uiState.value)
        assertEquals(listOf("token-1"), api.logoutTokens)
    }

    private suspend fun kotlinx.coroutines.test.TestScope.assertLateDoseIgnored(result: ApiResult<MedicationTakeDto>) {
        signIn(1)
        val model = model()
        runCurrent()
        api.delayDose = true
        model.recordDose(model.uiState.value.dashboardData.schedules.single())
        runCurrent()
        val oldDose = api.doseJob!!
        signIn(2)
        assertTrue(oldDose.isCancelled)
        runCurrent()
        val nextState = model.uiState.value
        val requests = api.peopleRequests.size

        api.doseContinuation!!.resume(result)
        runCurrent()

        assertTrue(api.oldDoseReturned)
        assertEquals(nextState, model.uiState.value)
        assertEquals(requests, api.peopleRequests.size)
    }

    private fun model() = DashboardViewModel(sessions, api).also { store.put("dashboard", it) }

    private fun signIn(account: Long, household: Long = 1, server: String = "https://one.example/") {
        sessions.saveSession(
            SessionPayload("token-$account", refreshToken = "refresh-$account", me = UserDto(id = account), household = HouseholdDto(id = household)),
            server
        )
    }

    private fun assertCleared(state: DashboardUiState) {
        assertEquals(DashboardData(), state.dashboardData)
        assertNull(state.takingScheduleId)
        assertNull(state.errorMessage)
        assertNull(state.actionSuccessMessage)
        assertFalse(state.isRefreshing)
    }

    private class ControlledApi : MedTrackerApi {
        val peopleRequests = mutableListOf<Pair<String, Long>>()
        var delayPeople = false
        var delayDose = false
        var delayLogout = false
        val logoutTokens = mutableListOf<String>()
        var peopleJob: Job? = null
        var doseJob: Job? = null
        var peopleContinuation: Continuation<ApiResult<List<PersonDto>>>? = null
        var doseContinuation: Continuation<ApiResult<MedicationTakeDto>>? = null
        var logoutContinuation: Continuation<ApiResult<Unit>>? = null
        var oldPeopleReturned = false
        var oldDoseReturned = false
        var doseRequests = 0

        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<PersonDto>> {
            peopleRequests.add(baseUrl to householdId)
            if (delayPeople && accessToken == "token-1") {
                peopleJob = currentCoroutineContext()[Job]
                val result = suspendCoroutine<ApiResult<List<PersonDto>>> { peopleContinuation = it }
                oldPeopleReturned = true
                return result
            }
            val id = accessToken.substringAfter("token-").toLong()
            return ApiResult.Success(listOf(PersonDto(id = id, name = "Person $id")))
        }

        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) =
            ApiResult.Success(listOf(MedicationDto(id = householdId, name = "Medication $accessToken")))
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) =
            ApiResult.Success(listOf(ScheduleDto(id = accessToken.substringAfter("token-").toLong())))
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) =
            ApiResult.Success(listOf(MedicationTakeDto(id = accessToken.substringAfter("token-").toLong())))
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload): ApiResult<MedicationTakeDto> {
            doseRequests += 1
            if (delayDose) {
                doseJob = currentCoroutineContext()[Job]
                val result = suspendCoroutine<ApiResult<MedicationTakeDto>> { doseContinuation = it }
                oldDoseReturned = true
                return result
            }
            return ApiResult.Success(MedicationTakeDto(id = 99, scheduleId = request.sourceId.toLong()))
        }

        override suspend fun logout(baseUrl: String, accessToken: String): ApiResult<Unit> {
            logoutTokens.add(accessToken)
            return if (delayLogout) suspendCoroutine { logoutContinuation = it } else ApiResult.Success(Unit)
        }
        override suspend fun exchangeOidc(baseUrl: String, request: OidcExchangeRequest): ApiResult<SessionPayload> = error("Not used")
        override suspend fun refresh(baseUrl: String, request: RefreshRequest): ApiResult<SessionPayload> = error("Not used")
    }
}
