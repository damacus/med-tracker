package io.damacus.medtracker.ui.reports

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException

data class ReportsUiState(
    val sessionRevision: String? = null,
    val isExporting: Boolean = false,
    val exportedFilePath: String? = null,
    val exportSuccessMessage: String? = null,
    val errorMessage: String? = null
) {
    fun forSession(session: AppSession): ReportsUiState =
        if (sessionRevision == session.revision) this else ReportsUiState(sessionRevision = session.revision)
}

class ReportsViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi(),
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val exportDirProvider: () -> File = { File(System.getProperty("java.io.tmpdir"), "medtracker-exports") }
) : ViewModel() {

    private val _uiState = MutableStateFlow(ReportsUiState(sessionRevision = sessionManager.sessionState.value.revision))
    val uiState: StateFlow<ReportsUiState> = _uiState.asStateFlow()
    private var sessionWork: Job = SupervisorJob(viewModelScope.coroutineContext[Job])
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            sessionWork.cancel()
            sessionWork = SupervisorJob(viewModelScope.coroutineContext[Job])
            _uiState.value = ReportsUiState(sessionRevision = session.revision)
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        sessionWork.cancel()
        super.onCleared()
    }

    fun clearMessages() {
        _uiState.update { it.copy(exportSuccessMessage = null, errorMessage = null) }
    }

    fun exportBackupZip() {
        val session = sessionManager.sessionState.value
        val householdId = session.household?.id ?: return
        val token = session.accessToken?.takeIf { it.isNotBlank() } ?: return
        val work = sessionWork

        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) {
                it.copy(isExporting = true, errorMessage = null, exportSuccessMessage = null)
            }
            when (val result = apiClient.exportPortableBackup(session.serverUrl, token, householdId)) {
                is ApiResult.Success -> {
                    if (!isCurrentSession(session, work)) return@launch
                    val export = result.data
                    val savedFile = try {
                        withContext(ioDispatcher) {
                            val dir = exportDirProvider().apply { mkdirs() }
                            val safeName = File(export.filename).name.ifBlank { "medtracker-backup-$householdId.zip" }
                            val target = File(dir, safeName)
                            target.writeBytes(export.bytes)
                            target
                        }
                    } catch (_: IOException) {
                        updateForSession(session, work) {
                            it.copy(
                                isExporting = false,
                                errorMessage = "Unable to save exported backup file."
                            )
                        }
                        return@launch
                    }
                    updateForSession(session, work) {
                        it.copy(
                            isExporting = false,
                            exportedFilePath = savedFile.absolutePath,
                            exportSuccessMessage = "Saved ${savedFile.name} (${export.bytes.size} bytes)"
                        )
                    }
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) {
                        it.copy(isExporting = false, errorMessage = result.message)
                    }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) {
                        it.copy(
                            isExporting = false,
                            errorMessage = "Network error exporting portable backup. Please try again."
                        )
                    }
                }
            }
        }
    }

    private fun isCurrentSession(session: AppSession, work: Job): Boolean =
        work.isActive && sessionManager.sessionState.value.revision == session.revision

    private fun updateForSession(session: AppSession, work: Job, update: (ReportsUiState) -> ReportsUiState) {
        _uiState.update { current ->
            if (isCurrentSession(session, work) && current.sessionRevision == session.revision) update(current) else current
        }
    }

    class Factory(
        private val sessionManager: SessionManager,
        private val exportDir: File
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return ReportsViewModel(sessionManager, exportDirProvider = { exportDir }) as T
        }
    }
}
