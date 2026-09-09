package io.damacus.medtracker.ui.healthevents

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
import io.damacus.medtracker.data.model.OidcExchangeRequest
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.RefreshRequest
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class HealthEventsViewModelTest {

    private val testDispatcher = StandardTestDispatcher()
    private lateinit var credentialStore: MemoryCredentialStore
    private lateinit var sessionManager: SessionManager

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
        credentialStore = MemoryCredentialStore()
        sessionManager = SessionManager(credentialStore)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun loadHealthEventsPopulatesUiState() = runTest {
        val fakeApi = FakeHealthEventsApi()
        val viewModel = HealthEventsViewModel(sessionManager, fakeApi)

        val sessionPayload = SessionPayload(
            accessToken = "token",
            accessTokenExpiresAt = "2026-12-31T23:59:59Z",
            refreshToken = "refresh",
            refreshTokenExpiresAt = "2027-12-31T23:59:59Z",
            me = UserDto(1L, "user@example.com", "User"),
            household = HouseholdDto(10L, "Test Household")
        )
        sessionManager.saveSession(sessionPayload, "http://localhost:3000")
        testDispatcher.scheduler.advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertEquals(1, state.events.size)
        assertEquals("Nausea", state.events.first().title)
    }

    private class FakeHealthEventsApi : MedTrackerApi {
        override suspend fun getHealthEvents(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HealthEventDto>> {
            return ApiResult.Success(listOf(HealthEventDto(id = 1L, eventKind = "suspected_side_effect", title = "Nausea", notes = "After dose")))
        }
        override suspend fun exchangeOidc(baseUrl: String, request: OidcExchangeRequest) = ApiResult.Error("stub", "stub")
        override suspend fun refresh(baseUrl: String, request: RefreshRequest) = ApiResult.Error("stub", "stub")
        override suspend fun logout(baseUrl: String, accessToken: String) = ApiResult.Success(Unit)
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<PersonDto>())
        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<MedicationDto>())
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<ScheduleDto>())
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<MedicationTakeDto>())
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload) = ApiResult.Error("stub", "stub")
        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<LocationDto>())
        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<HouseholdInvitationDto>())
        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String) = ApiResult.Error("stub", "stub")
        override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload) = ApiResult.Error("stub", "stub")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload) = ApiResult.Error("stub", "stub")
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload) = ApiResult.Error("stub", "stub")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = ApiResult.Error("stub", "stub")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = ApiResult.Error("stub", "stub")
    }

    private class MemoryCredentialStore : CredentialStore {
        private var stored: String? = null
        override fun read(): String? = stored
        override fun write(value: String) { stored = value }
        override fun clear() { stored = null }
    }
}
