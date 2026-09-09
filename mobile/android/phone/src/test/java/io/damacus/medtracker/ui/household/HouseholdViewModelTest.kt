package io.damacus.medtracker.ui.household

import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.HouseholdInvitationDto
import io.damacus.medtracker.data.model.LocationDto
import io.damacus.medtracker.data.model.OidcExchangeRequest
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.RefreshRequest
import io.damacus.medtracker.data.model.SessionPayload
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
class HouseholdViewModelTest {

    private val testDispatcher = StandardTestDispatcher()
    private lateinit var sessionManager: SessionManager
    private lateinit var mockApi: FakeApi
    private lateinit var viewModel: HouseholdViewModel

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
        sessionManager = SessionManager(FakeCredentialStore())
        sessionManager.saveSession(
            payload = SessionPayload(
                accessToken = "valid-token",
                refreshToken = "valid-refresh",
                household = HouseholdDto(id = 42L, name = "Test Household")
            )
        )
        mockApi = FakeApi()
        viewModel = HouseholdViewModel(sessionManager, mockApi)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun loadDataLoadsLocationsAndInvitations() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertEquals(1, state.locations.size)
        assertEquals("Cabinet", state.locations.first().name)
        assertEquals(1, state.invitations.size)
        assertEquals("user@example.com", state.invitations.first().email)
    }

    @Test
    fun createInvitationCallsApiAndUpdatesState() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()

        var calledBack = false
        viewModel.createInvitation("newuser@example.com", "member") {
            calledBack = true
        }
        testDispatcher.scheduler.advanceUntilIdle()

        val state = viewModel.uiState.value
        assertEquals(2, state.invitations.size)
        assertEquals("newuser@example.com", state.invitations.last().email)
        assertEquals("Invitation sent to newuser@example.com", state.successMessage)
        assert(calledBack)
    }

    private class FakeCredentialStore : CredentialStore {
        private var stored: String? = null
        override fun read(): String? = stored
        override fun write(value: String) { stored = value }
        override fun clear() { stored = null }
    }

    private class FakeApi : MedTrackerApi {
        val locationsList = mutableListOf(LocationDto(id = 1L, name = "Cabinet"))
        val invitationsList = mutableListOf(HouseholdInvitationDto(id = 1L, email = "user@example.com", role = "member"))

        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<LocationDto>> {
            return ApiResult.Success(locationsList)
        }

        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HouseholdInvitationDto>> {
            return ApiResult.Success(invitationsList)
        }

        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String): ApiResult<HouseholdInvitationDto> {
            val newInv = HouseholdInvitationDto(id = 2L, email = email, role = role)
            return ApiResult.Success(newInv)
        }

        override suspend fun exchangeOidc(baseUrl: String, request: OidcExchangeRequest) = error("Not used")
        override suspend fun refresh(baseUrl: String, request: RefreshRequest) = error("Not used")
        override suspend fun logout(baseUrl: String, accessToken: String) = error("Not used")
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload) = error("Not used")
        override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload) = error("Not used")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload) = error("Not used")
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload) = error("Not used")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = error("Not used")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = error("Not used")
    }
}
