package io.damacus.medtracker.ui.lookup

import androidx.lifecycle.ViewModelStore
import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.MedicationLookupResultDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.SessionPayload
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

@OptIn(ExperimentalCoroutinesApi::class)
class MedicationLookupViewModelTest {
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

    private fun signIn(household: Long) {
        sessions.saveSession(
            SessionPayload("token-$household", refreshToken = "refresh-$household", household = HouseholdDto(id = household, name = "HH $household")),
            "https://one.example/"
        )
    }

    @Test fun `logout and household switch clear lookup query and results`() = runTest(dispatcher) {
        signIn(1)
        val model = MedicationLookupViewModel(sessions, api).also { store.put("lookup", it) }
        model.search("ibuprofen")
        advanceTimeBy(400)
        runCurrent()
        assertEquals(1, model.uiState.value.results.size)

        sessions.clearSession()
        assertEquals("", model.uiState.value.query)
        assertTrue(model.uiState.value.results.isEmpty())
    }

    @Test fun `stale lookup response cannot repopulate after household switch`() = runTest(dispatcher) {
        signIn(1)
        api.delayLookup = true
        val model = MedicationLookupViewModel(sessions, api).also { store.put("lookup", it) }
        model.search("ibuprofen")
        advanceTimeBy(400)
        runCurrent()
        val pending = api.pending!!

        api.delayLookup = false
        signIn(2)
        runCurrent()
        val stateAfterSwitch = model.uiState.value
        assertTrue(stateAfterSwitch.results.isEmpty())

        pending.resume(ApiResult.Success(listOf(MedicationLookupResultDto(name = "Stale Med"))))
        runCurrent()

        assertTrue(api.returned)
        assertEquals(stateAfterSwitch, model.uiState.value)
        assertTrue(model.uiState.value.results.isEmpty())
    }

    private class ControlledApi : MedTrackerApi {
        var delayLookup = false
        var pending: Continuation<ApiResult<List<MedicationLookupResultDto>>>? = null
        var returned = false

        override suspend fun lookupMedication(baseUrl: String, accessToken: String, householdId: Long, query: String): ApiResult<List<MedicationLookupResultDto>> {
            if (delayLookup) {
                val res = suspendCoroutine<ApiResult<List<MedicationLookupResultDto>>> { pending = it }
                returned = true
                return res
            }
            return ApiResult.Success(listOf(MedicationLookupResultDto(name = "Result-$householdId-$query")))
        }

        override suspend fun logout(baseUrl: String, accessToken: String) = error("Not used")
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload) = error("Not used")
        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long) = error("Not used")
        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String) = error("Not used")
        override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload) = error("Not used")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload) = error("Not used")
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload) = error("Not used")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = error("Not used")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = error("Not used")
    }
}
