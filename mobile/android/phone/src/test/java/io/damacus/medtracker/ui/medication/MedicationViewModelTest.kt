package io.damacus.medtracker.ui.medication

import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.MedicationDto
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
class MedicationViewModelTest {

    private val testDispatcher = StandardTestDispatcher()
    private lateinit var sessionManager: SessionManager
    private lateinit var mockApi: FakeApi
    private lateinit var viewModel: MedicationViewModel

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
        viewModel = MedicationViewModel(sessionManager, mockApi)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun loadMedicationsUpdatesStateWithApiData() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertEquals(2, state.medications.size)
        assertEquals("Paracetamol", state.medications.first().name)
    }

    @Test
    fun updateSearchQueryFiltersMedications() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()

        viewModel.updateSearchQuery("Ibu")
        val state = viewModel.uiState.value

        assertEquals(1, state.filteredMedications.size)
        assertEquals("Ibuprofen", state.filteredMedications.first().name)
    }

    @Test
    fun createMedicationCallsApiAndAppendsToState() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()

        var calledBack = false
        val payload = CreateMedicationPayload(name = "Aspirin", category = "Pain")
        viewModel.createMedication(payload, locationId = 1) {
            calledBack = true
        }
        testDispatcher.scheduler.advanceUntilIdle()

        val state = viewModel.uiState.value
        assertEquals(3, state.medications.size)
        assertEquals("Aspirin", state.medications.last().name)
        assertEquals("Added Aspirin", state.successMessage)
        assert(calledBack)
    }

    @Test
    fun recordStockRemovalCallsApiAndUpdatesMedicationState() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()

        var calledBack = false
        viewModel.recordStockRemoval(medicationId = 1L, quantity = 2.0, reason = "damaged") {
            calledBack = true
        }
        testDispatcher.scheduler.advanceUntilIdle()

        val state = viewModel.uiState.value
        assertEquals("Stock updated for Paracetamol", state.successMessage)
        assert(calledBack)
    }

    private class FakeCredentialStore : CredentialStore {
        private var stored: String? = null
        override fun read(): String? = stored
        override fun write(value: String) { stored = value }
        override fun clear() { stored = null }
    }

    private class FakeApi : MedTrackerApi {
        val medsList = mutableListOf(
            MedicationDto(id = 1L, name = "Paracetamol", category = "Pain"),
            MedicationDto(id = 2L, name = "Ibuprofen", category = "Pain")
        )

        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationDto>> {
            return ApiResult.Success(medsList)
        }

        override suspend fun createMedication(
            baseUrl: String,
            accessToken: String,
            householdId: Long,
            locationId: Int,
            request: CreateMedicationPayload
        ): ApiResult<MedicationDto> {
            val newMed = MedicationDto(id = 3L, name = request.name, category = request.category)
            return ApiResult.Success(newMed)
        }

        override suspend fun exchangeOidc(baseUrl: String, request: OidcExchangeRequest) = error("Not used")
        override suspend fun refresh(baseUrl: String, request: RefreshRequest) = error("Not used")
        override suspend fun logout(baseUrl: String, accessToken: String) = error("Not used")
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload) = error("Not used")
        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String) = error("Not used")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload): ApiResult<MedicationDto> {
            val existing = medsList.find { it.id == request.medicationId } ?: return ApiResult.Error("not_found", "Not found")
            val updated = existing.copy(currentSupply = (existing.currentSupply ?: 10.0) - request.quantity)
            return ApiResult.Success(updated)
        }
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload) = error("Not used")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = error("Not used")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = error("Not used")
    }
}
