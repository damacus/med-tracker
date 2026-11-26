package io.damacus.medtracker.ui.reports

import androidx.lifecycle.ViewModelStore
import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.api.PortableBackupExportDto
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.SessionPayload
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.ByteArrayOutputStream
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
class ReportsViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val store = ViewModelStore()
    private val sessions = SessionManager(object : CredentialStore {
        private var saved: String? = null
        override fun read() = saved
        override fun write(value: String) { saved = value }
        override fun clear() { saved = null }
    })

    private class FakeReportsApi : MedTrackerApi {
        var exportHandler: suspend (String, String, Long) -> ApiResult<PortableBackupExportDto> = { _, _, _ ->
            ApiResult.Error("not_stubbed", "Not stubbed")
        }
        val calls = mutableListOf<Triple<String, String, Long>>()

        override suspend fun exportPortableBackup(
            baseUrl: String,
            accessToken: String,
            householdId: Long
        ): ApiResult<PortableBackupExportDto> {
            calls.add(Triple(baseUrl, accessToken, householdId))
            return exportHandler(baseUrl, accessToken, householdId)
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

    @Before fun setUp() { Dispatchers.setMain(dispatcher) }
    @After fun tearDown() {
        store.clear()
        Dispatchers.resetMain()
    }

    private fun signIn(householdId: Long) {
        sessions.saveSession(
            SessionPayload("tok-$householdId", refreshToken = "ref-$householdId", household = HouseholdDto(householdId, "HH $householdId")),
            "http://localhost:5150/"
        )
    }

    @Test
    fun exportBackupZipStagesPendingExportAndWritesExactDecodedZipBytesViaWriter() = runTest(dispatcher) {
        signIn(12L)
        val zipBytes = byteArrayOf(0x50, 0x4B, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00)
        val api = FakeReportsApi().apply {
            exportHandler = { _, _, _ ->
                ApiResult.Success(PortableBackupExportDto("../household-12.zip", "application/zip", zipBytes))
            }
        }
        val viewModel = ReportsViewModel(sessions, api, dispatcher).also { store.put("reports", it) }

        viewModel.exportBackupZip()
        runCurrent()

        assertEquals(listOf(Triple("http://localhost:5150/", "tok-12", 12L)), api.calls)
        val staged = viewModel.uiState.value
        assertFalse(staged.isExporting)
        assertNull(staged.errorMessage)
        assertNotNull(staged.pendingExport)
        assertEquals("household-12.zip", staged.pendingExport?.filename)
        assertArrayEquals(zipBytes, staged.pendingExport?.bytes)

        val output = ByteArrayOutputStream()
        viewModel.savePendingExport("content://documents/household-12.zip") { bytes ->
            output.write(bytes)
        }
        runCurrent()

        assertArrayEquals(zipBytes, output.toByteArray())
        val saved = viewModel.uiState.value
        assertFalse(saved.isExporting)
        assertNull(saved.pendingExport)
        assertNull(saved.errorMessage)
        assertTrue(saved.exportSuccessMessage?.contains("household-12.zip") == true)
        assertEquals("content://documents/household-12.zip", saved.exportedFilePath)
    }

    @Test
    fun cancelPendingExportClearsStagedExportWithoutError() = runTest(dispatcher) {
        signIn(12L)
        val api = FakeReportsApi().apply {
            exportHandler = { _, _, _ ->
                ApiResult.Success(PortableBackupExportDto("backup.zip", "application/zip", byteArrayOf(0x50, 0x4B)))
            }
        }
        val viewModel = ReportsViewModel(sessions, api, dispatcher).also { store.put("reports-cancel", it) }

        viewModel.exportBackupZip()
        runCurrent()
        assertNotNull(viewModel.uiState.value.pendingExport)

        viewModel.cancelPendingExport()
        assertNull(viewModel.uiState.value.pendingExport)
        assertNull(viewModel.uiState.value.errorMessage)
        assertFalse(viewModel.uiState.value.isExporting)
    }

    @Test
    fun savePendingExportSurfacesWriterIoFailure() = runTest(dispatcher) {
        signIn(12L)
        val api = FakeReportsApi().apply {
            exportHandler = { _, _, _ ->
                ApiResult.Success(PortableBackupExportDto("backup.zip", "application/zip", byteArrayOf(0x50, 0x4B)))
            }
        }
        val viewModel = ReportsViewModel(sessions, api, dispatcher).also { store.put("reports-io", it) }

        viewModel.exportBackupZip()
        runCurrent()

        viewModel.savePendingExport("content://documents/backup.zip") {
            throw IOException("disk full")
        }
        runCurrent()

        assertFalse(viewModel.uiState.value.isExporting)
        assertNull(viewModel.uiState.value.pendingExport)
        assertNotNull(viewModel.uiState.value.errorMessage)
    }

    @Test
    fun exportBackupZipSurfacesApiAndNetworkErrors() = runTest(dispatcher) {
        signIn(12L)
        val api = FakeReportsApi().apply {
            exportHandler = { _, _, _ -> ApiResult.Error("forbidden", "Admin access required", statusCode = 403) }
        }
        val viewModel = ReportsViewModel(sessions, api, dispatcher).also { store.put("reports-err", it) }

        viewModel.exportBackupZip()
        runCurrent()

        assertEquals("Admin access required", viewModel.uiState.value.errorMessage)
        assertNull(viewModel.uiState.value.pendingExport)
        assertNull(viewModel.uiState.value.exportSuccessMessage)

        api.exportHandler = { _, _, _ -> ApiResult.NetworkError(IOException("offline")) }
        viewModel.exportBackupZip()
        runCurrent()

        assertNotNull(viewModel.uiState.value.errorMessage)
        assertFalse(viewModel.uiState.value.isExporting)
    }

    @Test
    fun logoutClearsReportsStateAndIgnoresInFlightExport() = runTest(dispatcher) {
        signIn(12L)
        val deferred = CompletableDeferred<ApiResult<PortableBackupExportDto>>()
        val api = FakeReportsApi().apply {
            exportHandler = { _, _, _ -> deferred.await() }
        }
        val viewModel = ReportsViewModel(sessions, api, dispatcher).also { store.put("reports-logout", it) }

        viewModel.exportBackupZip()
        runCurrent()
        assertTrue(viewModel.uiState.value.isExporting)

        sessions.clearSession()
        assertFalse(viewModel.uiState.value.isExporting)
        assertNull(viewModel.uiState.value.pendingExport)

        deferred.complete(ApiResult.Success(PortableBackupExportDto("stale.zip", "application/zip", byteArrayOf(1, 2))))
        runCurrent()

        assertNull(viewModel.uiState.value.pendingExport)
        assertNull(viewModel.uiState.value.exportedFilePath)
        assertNull(viewModel.uiState.value.exportSuccessMessage)
    }
}
