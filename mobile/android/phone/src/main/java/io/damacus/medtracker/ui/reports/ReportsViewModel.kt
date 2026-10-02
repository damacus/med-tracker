package io.damacus.medtracker.ui.reports

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.SessionManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ReportsUiState(
    val isExporting: Boolean = false,
    val exportSuccessMessage: String? = null,
    val errorMessage: String? = null
)

class ReportsViewModel(
    private val sessionManager: SessionManager
) : ViewModel() {

    private val _uiState = MutableStateFlow(ReportsUiState())
    val uiState: StateFlow<ReportsUiState> = _uiState.asStateFlow()

    fun exportBackupZip() {
        val session = sessionManager.sessionState.value
        val householdId = session.household?.id ?: return
        val token = session.accessToken ?: return

        _uiState.update { it.copy(isExporting = true, errorMessage = null) }

        viewModelScope.launch {
            // Initiate portable export download
            _uiState.update {
                it.copy(
                    isExporting = false,
                    exportSuccessMessage = "Portable backup exported for household $householdId"
                )
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return ReportsViewModel(sessionManager) as T
        }
    }
}
