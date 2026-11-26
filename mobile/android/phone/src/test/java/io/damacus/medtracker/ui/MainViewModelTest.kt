package io.damacus.medtracker.ui

import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdChoice
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
import kotlinx.coroutines.CompletableDeferred
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
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class MainViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    private class InMemoryCredentialStore : CredentialStore {
        var stored: String? = null
        override fun read(): String? = stored
        override fun write(value: String) {
            stored = value
        }
        override fun clear() {
            stored = null
        }
    }

    private class FakeMainApi : MedTrackerApi {
        var householdsResult: ApiResult<List<HouseholdChoice>> = ApiResult.Success(emptyList())
        var meHandler: suspend (String, String, Long) -> ApiResult<UserDto> = { _, _, _ ->
            ApiResult.Error("not_stubbed", "Not stubbed")
        }
        val meCalls = mutableListOf<Triple<String, String, Long>>()

        override suspend fun getHouseholds(baseUrl: String, accessToken: String): ApiResult<List<HouseholdChoice>> =
            householdsResult

        override suspend fun getMe(baseUrl: String, accessToken: String, householdId: Long): ApiResult<UserDto> {
            meCalls.add(Triple(baseUrl, accessToken, householdId))
            return meHandler(baseUrl, accessToken, householdId)
        }

        override suspend fun logout(baseUrl: String, accessToken: String) = ApiResult.Success(Unit)
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<PersonDto>())
        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<MedicationDto>())
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<ScheduleDto>())
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<MedicationTakeDto>())
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload) = ApiResult.Error("unused", "unused")
        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<LocationDto>())
        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<HouseholdInvitationDto>())
        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String) = ApiResult.Error("unused", "unused")
        override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload) = ApiResult.Error("unused", "unused")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload) = ApiResult.Error("unused", "unused")
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload) = ApiResult.Error("unused", "unused")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = ApiResult.Error("unused", "unused")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = ApiResult.Error("unused", "unused")
    }

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun selectHouseholdFetchesCurrentUserAndPersistsIntoSessionManager() = runTest(dispatcher) {
        val store = InMemoryCredentialStore()
        val sessionManager = SessionManager(store)
        val api = FakeMainApi().apply {
            householdsResult = ApiResult.Success(
                listOf(
                    HouseholdChoice(10L, "Family Home", "administrator"),
                    HouseholdChoice(20L, "Grandparents", "member")
                )
            )
            meHandler = { _, _, householdId ->
                ApiResult.Success(
                    UserDto(
                        id = 99L,
                        emailAddress = "damacus@example.com",
                        name = "Damacus",
                        role = if (householdId == 10L) "administrator" else "member"
                    )
                )
            }
        }
        val viewModel = MainViewModel(sessionManager, api)

        viewModel.completeMobileLogin(
            SessionPayload(accessToken = "tok-1", refreshToken = "ref-1", oauthState = "{}"),
            "http://10.0.2.2:5150/"
        )
        runCurrent()

        viewModel.selectHousehold(10L)
        val revisionAfterSelection = sessionManager.sessionState.value.revision
        runCurrent()

        assertEquals(listOf(Triple("http://10.0.2.2:5150/", "tok-1", 10L)), api.meCalls)
        assertEquals(revisionAfterSelection, sessionManager.sessionState.value.revision)
        assertEquals("Damacus", sessionManager.sessionState.value.user?.name)
        assertEquals("damacus@example.com", sessionManager.sessionState.value.user?.emailAddress)
        assertEquals("administrator", sessionManager.sessionState.value.user?.role)

        val restored = SessionManager(store)
        assertEquals("Damacus", restored.sessionState.value.user?.name)
    }

    @Test
    fun startupRestoresCurrentUserWhenSessionHasHouseholdButNullMe() = runTest(dispatcher) {
        val store = InMemoryCredentialStore()
        SessionManager(store).saveSession(
            SessionPayload(
                accessToken = "tok-restored",
                refreshToken = "ref-restored",
                household = HouseholdDto(15L, "Restored Home"),
                me = null
            ),
            serverUrl = "http://10.0.2.2:5150/"
        )
        val sessionManager = SessionManager(store)
        val api = FakeMainApi().apply {
            meHandler = { _, _, _ ->
                ApiResult.Success(UserDto(7L, "restored@example.com", "Restored User", "owner"))
            }
        }

        MainViewModel(sessionManager, api)
        runCurrent()

        assertEquals(listOf(Triple("http://10.0.2.2:5150/", "tok-restored", 15L)), api.meCalls)
        assertEquals("Restored User", sessionManager.sessionState.value.user?.name)
        assertEquals("restored@example.com", sessionManager.sessionState.value.user?.emailAddress)
        assertEquals("owner", sessionManager.sessionState.value.user?.role)
    }

    @Test
    fun inFlightGetMeResultIsIgnoredIfUserLogsOutBeforeCompletion() = runTest(dispatcher) {
        val store = InMemoryCredentialStore()
        val sessionManager = SessionManager(store)
        val meDeferred = CompletableDeferred<ApiResult<UserDto>>()
        val api = FakeMainApi().apply {
            householdsResult = ApiResult.Success(listOf(HouseholdChoice(10L, "Family Home", "owner")))
            meHandler = { _, _, _ -> meDeferred.await() }
        }
        val viewModel = MainViewModel(sessionManager, api)

        viewModel.completeMobileLogin(
            SessionPayload(accessToken = "tok-1", refreshToken = "ref-1", oauthState = "{}"),
            "http://10.0.2.2:5150/"
        )
        runCurrent()
        assertEquals(10L, sessionManager.sessionState.value.household?.id)

        viewModel.logout()
        runCurrent()

        meDeferred.complete(ApiResult.Success(UserDto(1L, "stale@example.com", "Stale", "owner")))
        runCurrent()

        assertNull(sessionManager.sessionState.value.sessionPayload)
        assertNull(sessionManager.sessionState.value.user)
    }
}
