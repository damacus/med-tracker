package io.damacus.medtracker.ui.medication

import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.SessionPayload
import kotlinx.coroutines.CompletableDeferred
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
        viewModel.createMedication(payload, locationId = 1, expectedRevision = sessionManager.sessionState.value.revision) {
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

    @Test
    fun householdSwitchClearsPreviousMedicationListBeforeNewLoad() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()
        mockApi.pendingMedicationLoad = CompletableDeferred()
        sessionManager.saveSession(SessionPayload(accessToken = "other-token", refreshToken = "other-refresh", household = HouseholdDto(43L, "Other household")))
        val immediate = viewModel.uiState.value.medications
        mockApi.pendingMedicationLoad?.complete(ApiResult.Success(emptyList()))
        testDispatcher.scheduler.advanceUntilIdle()
        assertEquals(emptyList<MedicationDto>(), immediate)
    }

    @Test
    fun latePreviousHouseholdLoadCannotReplaceCurrentMedicationList() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()
        val pending = CompletableDeferred<ApiResult<List<MedicationDto>>>()
        mockApi.pendingMedicationLoad = pending
        viewModel.loadMedications()
        testDispatcher.scheduler.runCurrent()
        mockApi.pendingMedicationLoad = null
        sessionManager.saveSession(SessionPayload(accessToken = "other-token", refreshToken = "other-refresh", household = HouseholdDto(43L, "Other household")))
        testDispatcher.scheduler.runCurrent()
        pending.complete(ApiResult.Success(listOf(MedicationDto(id = 99L, name = "Previous household secret"))))
        testDispatcher.scheduler.advanceUntilIdle()
        assertEquals(listOf("Other household medicine"), viewModel.uiState.value.medications.map { it.name })
    }

    @Test
    fun lateCreateCannotPopulateNewHouseholdOrCompleteOldForm() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()
        val pending = CompletableDeferred<ApiResult<MedicationDto>>()
        mockApi.pendingCreate = pending
        var completed = false
        viewModel.createMedication(CreateMedicationPayload(name = "Previous household created medicine"), 7, sessionManager.sessionState.value.revision) { completed = true }
        testDispatcher.scheduler.runCurrent()
        sessionManager.saveSession(SessionPayload(accessToken = "other-token", refreshToken = "other-refresh", household = HouseholdDto(43L, "Other household")))
        testDispatcher.scheduler.runCurrent()
        pending.complete(ApiResult.Success(MedicationDto(id = 99L, name = "Previous household created medicine")))
        testDispatcher.scheduler.advanceUntilIdle()
        assertEquals(listOf(42L), mockApi.createHouseholds)
        assertEquals(listOf("Other household medicine"), viewModel.uiState.value.medications.map { it.name })
        assertFalse(completed)
    }

    @Test
    fun authenticatedAccountWithoutHouseholdClearsPreviousMedicationState() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()
        sessionManager.saveSession(SessionPayload(accessToken = "account-token", refreshToken = "account-refresh", household = null))
        testDispatcher.scheduler.advanceUntilIdle()
        assertEquals(emptyList<MedicationDto>(), viewModel.uiState.value.medications)
        assertEquals(sessionManager.sessionState.value.revision, viewModel.uiState.value.sessionRevision)
    }

    @Test
    fun staleFormCannotCreateInNewHousehold() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()
        val originalRevision = sessionManager.sessionState.value.revision
        sessionManager.saveSession(SessionPayload(accessToken = "other-token", refreshToken = "other-refresh", household = HouseholdDto(43L, "Other household")))
        var completed = false
        viewModel.createMedication(CreateMedicationPayload(name = "Stale form"), null, originalRevision) { completed = true }
        testDispatcher.scheduler.advanceUntilIdle()
        assertEquals(emptyList<Long>(), mockApi.createHouseholds)
        assertFalse(completed)
    }

    @Test
    fun noLabelCreationKeepsOriginalHouseholdAndNullLabel() = runTest(testDispatcher) {
        testDispatcher.scheduler.advanceUntilIdle()
        viewModel.createMedication(CreateMedicationPayload(name = "No label"), null, sessionManager.sessionState.value.revision) {}
        testDispatcher.scheduler.advanceUntilIdle()
        assertEquals(listOf(42L), mockApi.createHouseholds)
        assertEquals(listOf<Int?>(null), mockApi.createLocations)
    }

    private class FakeCredentialStore : CredentialStore {
        private var stored: String? = null
        override fun read(): String? = stored
        override fun write(value: String) { stored = value }
        override fun clear() { stored = null }
    }

    private class FakeApi : MedTrackerApi {
        var pendingMedicationLoad: CompletableDeferred<ApiResult<List<MedicationDto>>>? = null
        var pendingCreate: CompletableDeferred<ApiResult<MedicationDto>>? = null
        val createHouseholds = mutableListOf<Long>()
        val createLocations = mutableListOf<Int?>()
        val medsList = mutableListOf(
            MedicationDto(id = 1L, name = "Paracetamol", category = "Pain"),
            MedicationDto(id = 2L, name = "Ibuprofen", category = "Pain")
        )

        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationDto>> {
            pendingMedicationLoad?.let { return it.await() }
            return ApiResult.Success(if (householdId == 42L) medsList else listOf(MedicationDto(id = 43L, name = "Other household medicine")))
        }

        override suspend fun createMedication(
            baseUrl: String,
            accessToken: String,
            householdId: Long,
            locationId: Int?,
            request: CreateMedicationPayload
        ): ApiResult<MedicationDto> {
            createHouseholds.add(householdId)
            createLocations.add(locationId)
            pendingCreate?.let { return it.await() }
            val newMed = MedicationDto(id = 3L, name = request.name, category = request.category)
            return ApiResult.Success(newMed)
        }


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
