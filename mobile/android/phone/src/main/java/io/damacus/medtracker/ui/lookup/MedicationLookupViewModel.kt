package io.damacus.medtracker.ui.lookup

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.MedicationLookupResultDto
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class MedicationLookupUiState(
    val query: String = "",
    val isLoading: Boolean = false,
    val results: List<MedicationLookupResultDto> = emptyList(),
    val errorMessage: String? = null
)

class MedicationLookupViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(MedicationLookupUiState())
    val uiState: StateFlow<MedicationLookupUiState> = _uiState.asStateFlow()
    private var searchJob: Job? = null

    fun search(query: String) {
        _uiState.update { it.copy(query = query) }
        if (query.length < 2) {
            _uiState.update { it.copy(results = emptyList(), isLoading = false) }
            return
        }

        searchJob?.cancel()
        searchJob = viewModelScope.launch {
            delay(300) // Debounce typing
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            val session = sessionManager.sessionState.value
            val householdId = session.household?.id ?: return@launch
            val token = session.accessToken ?: return@launch

            when (val res = apiClient.lookupMedication(session.serverUrl, token, householdId, query)) {
                is ApiResult.Success -> {
                    _uiState.update { it.copy(isLoading = false, results = res.data) }
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = res.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error during lookup") }
                }
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return MedicationLookupViewModel(sessionManager) as T
        }
    }
}
