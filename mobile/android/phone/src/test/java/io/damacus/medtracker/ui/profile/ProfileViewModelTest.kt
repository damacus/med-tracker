package io.damacus.medtracker.ui.profile

import androidx.lifecycle.ViewModelStore
import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.api.NotificationPreferencesDto
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
class ProfileViewModelTest {

    private val testDispatcher = StandardTestDispatcher()
    private val store = ViewModelStore()
    private lateinit var sessionManager: SessionManager

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
        sessionManager = SessionManager(object : CredentialStore {
            private var saved: String? = null
            override fun read(): String? = saved
            override fun write(value: String) { saved = value }
            override fun clear() { saved = null }
        })
    }

    @After
    fun tearDown() {
        store.clear()
        Dispatchers.resetMain()
    }

    private fun signIn(householdId: Long = 42L) {
        sessionManager.saveSession(
            SessionPayload(
                accessToken = "token-$householdId",
                refreshToken = "refresh-$householdId",
                me = UserDto(id = 1L, emailAddress = "alex@example.test", name = "Alex", role = "admin"),
                household = HouseholdDto(id = householdId, name = "Home $householdId")
            ),
            "http://localhost:5150/"
        )
    }

    @Test
    fun loadNotificationPreferencesPopulatesStateOnSuccess() = runTest(testDispatcher) {
        val expected = NotificationPreferencesDto(
            enabled = true,
            doseDueEnabled = true,
            missedDoseEnabled = false,
            lowStockEnabled = true,
            privateTextEnabled = true,
            morningTime = "08:00:00"
        )
        val fakeApi = StubMedTrackerApi(
            getPreferences = { ApiResult.Success(expected) }
        )

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertNull(state.errorMessage)
        assertEquals(expected, state.notificationPreferences)
        assertEquals(expected, state.confirmedPreferences)
    }

    @Test
    fun loadNotificationPreferencesReportsErrorOnFailure() = runTest(testDispatcher) {
        val fakeApi = StubMedTrackerApi(
            getPreferences = { ApiResult.Error("server_error", "Server unavailable", 500) }
        )

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertEquals("Server unavailable", state.errorMessage)
    }

    @Test
    fun loadNotificationPreferencesPopulatesEditableDefaultsOn404() = runTest(testDispatcher) {
        val fakeApi = StubMedTrackerApi(
            getPreferences = { ApiResult.Error("not_found", "Notification preference not found", 404) }
        )

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertNull(state.errorMessage)
        assertEquals(NotificationPreferencesDto(), state.notificationPreferences)
        assertNull(state.confirmedPreferences)
    }

    @Test
    fun loadNotificationPreferencesReportsErrorOn403Forbidden() = runTest(testDispatcher) {
        val fakeApi = StubMedTrackerApi(
            getPreferences = { ApiResult.Error("forbidden", "Forbidden", 403) }
        )

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertNull(state.notificationPreferences)
        assertEquals("Forbidden", state.errorMessage)
    }

    @Test
    fun loadNotificationPreferencesReportsErrorOnNetworkFailure() = runTest(testDispatcher) {
        val fakeApi = StubMedTrackerApi(
            getPreferences = { ApiResult.NetworkError(IOException("No internet")) }
        )

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isLoading)
        assertNull(state.notificationPreferences)
        assertEquals("Unable to load notification preferences. Check your connection.", state.errorMessage)
    }

    @Test
    fun updateNotificationPreferencesPersistsAndUpdatesState() = runTest(testDispatcher) {
        var savedPreferences: NotificationPreferencesDto? = null
        val initial = NotificationPreferencesDto(enabled = true, doseDueEnabled = true, lowStockEnabled = true)
        val updated = initial.copy(lowStockEnabled = false, privateTextEnabled = true)

        val fakeApi = StubMedTrackerApi(
            getPreferences = { ApiResult.Success(initial) },
            updatePreferences = { prefs ->
                savedPreferences = prefs
                ApiResult.Success(prefs)
            }
        )

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()

        viewModel.updateNotificationPreferences(updated)
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isSaving)
        assertNull(state.errorMessage)
        assertEquals(updated, savedPreferences)
        assertEquals(updated, state.notificationPreferences)
        assertTrue(state.statusMessage?.contains("saved", ignoreCase = true) == true)
    }

    @Test
    fun updateNotificationPreferencesRevertsAndReportsErrorOnFailure() = runTest(testDispatcher) {
        val initial = NotificationPreferencesDto(enabled = true, doseDueEnabled = true)
        val attempted = initial.copy(enabled = false)

        val fakeApi = StubMedTrackerApi(
            getPreferences = { ApiResult.Success(initial) },
            updatePreferences = { ApiResult.NetworkError(IOException("Offline")) }
        )

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()

        viewModel.updateNotificationPreferences(attempted)
        advanceUntilIdle()

        val state = viewModel.uiState.value
        assertFalse(state.isSaving)
        assertEquals(initial, state.notificationPreferences)
        assertEquals("Unable to save notification preferences. Check your connection.", state.errorMessage)
    }

    @Test
    fun logoutClearsNotificationPreferencesSynchronously() = runTest(testDispatcher) {
        val initial = NotificationPreferencesDto(enabled = true, doseDueEnabled = true)
        val fakeApi = StubMedTrackerApi(getPreferences = { ApiResult.Success(initial) })

        signIn()
        val viewModel = ProfileViewModel(sessionManager, fakeApi).also { store.put("profile", it) }
        advanceUntilIdle()
        assertEquals(initial, viewModel.uiState.value.notificationPreferences)

        sessionManager.clearSession()
        assertNull(viewModel.uiState.value.notificationPreferences)
        assertNull(viewModel.uiState.value.errorMessage)
    }

    private class StubMedTrackerApi(
        private val getPreferences: suspend () -> ApiResult<NotificationPreferencesDto>,
        private val updatePreferences: suspend (NotificationPreferencesDto) -> ApiResult<NotificationPreferencesDto> = { ApiResult.Success(it) }
    ) : MedTrackerApi {
        override suspend fun getNotificationPreferences(baseUrl: String, accessToken: String, householdId: Long) = getPreferences()
        override suspend fun updateNotificationPreferences(baseUrl: String, accessToken: String, householdId: Long, preferences: NotificationPreferencesDto) = updatePreferences(preferences)
        override suspend fun logout(baseUrl: String, accessToken: String) = ApiResult.Success(Unit)
        override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<io.damacus.medtracker.data.model.PersonDto>())
        override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<io.damacus.medtracker.data.model.MedicationDto>())
        override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<io.damacus.medtracker.data.model.ScheduleDto>())
        override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<io.damacus.medtracker.data.model.MedicationTakeDto>())
        override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload) = ApiResult.Error("unused", "unused")
        override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<io.damacus.medtracker.data.model.LocationDto>())
        override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long) = ApiResult.Success(emptyList<io.damacus.medtracker.data.model.HouseholdInvitationDto>())
        override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String) = ApiResult.Error("unused", "unused")
        override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload) = ApiResult.Error("unused", "unused")
        override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload) = ApiResult.Error("unused", "unused")
        override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload) = ApiResult.Error("unused", "unused")
        override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = ApiResult.Error("unused", "unused")
        override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = ApiResult.Error("unused", "unused")
    }
}
