package io.damacus.medtracker.ui.admin

import androidx.lifecycle.ViewModelStore
import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdAdminSettingsDto
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.SessionPayload
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
class AdministrationViewModelTest {
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

    @Test fun `logout and household switch synchronously clear admin settings`() = runTest(dispatcher) {
        signIn(1)
        val model = AdministrationViewModel(sessions, api).also { store.put("admin", it) }
        runCurrent()
        assertEquals("plan-1", model.uiState.value.adminSettings?.subscriptionPlan)

        sessions.clearSession()
        assertNull(model.uiState.value.adminSettings)
        assertNull(model.uiState.value.errorMessage)

        signIn(2)
        assertNull(model.uiState.value.adminSettings)
        runCurrent()
        assertEquals("plan-2", model.uiState.value.adminSettings?.subscriptionPlan)
    }

    @Test fun `stale admin settings response cannot overwrite switched household`() = runTest(dispatcher) {
        signIn(1)
        api.delayHousehold = 1L
        val model = AdministrationViewModel(sessions, api).also { store.put("admin", it) }
        runCurrent()
        val pending = api.pending!!

        api.delayHousehold = null
        signIn(2)
        runCurrent()
        val stateAfterSwitch = model.uiState.value
        assertEquals("plan-2", stateAfterSwitch.adminSettings?.subscriptionPlan)

        pending.resume(ApiResult.Success(HouseholdAdminSettingsDto("stale-plan-1")))
        runCurrent()

        assertTrue(api.returned)
        assertEquals(stateAfterSwitch, model.uiState.value)
        assertEquals("plan-2", model.uiState.value.adminSettings?.subscriptionPlan)
    }

    private class ControlledApi : MedTrackerApi {
        var delayHousehold: Long? = null
        var pending: Continuation<ApiResult<HouseholdAdminSettingsDto>>? = null
        var returned = false

        override suspend fun getAdminSettings(baseUrl: String, accessToken: String, householdId: Long): ApiResult<HouseholdAdminSettingsDto> {
            if (delayHousehold == householdId) {
                val res = suspendCoroutine<ApiResult<HouseholdAdminSettingsDto>> { pending = it }
                returned = true
                return res
            }
            return ApiResult.Success(HouseholdAdminSettingsDto("plan-$householdId"))
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
