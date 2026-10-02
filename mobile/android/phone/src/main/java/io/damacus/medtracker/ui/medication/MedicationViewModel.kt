package io.damacus.medtracker.ui.medication

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.OffsetDateTime

data class MedicationUiState(
    val isLoading: Boolean = false,
    val medications: List<MedicationDto> = emptyList(),
    val searchQuery: String = "",
    val errorMessage: String? = null,
    val successMessage: String? = null
) {
    val filteredMedications: List<MedicationDto>
        get() = if (searchQuery.isBlank()) {
            medications
        } else {
            medications.filter {
                it.name.contains(searchQuery, ignoreCase = true) ||
                    it.category?.contains(searchQuery, ignoreCase = true) == true
            }
        }
}

class MedicationViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(MedicationUiState())
    val uiState: StateFlow<MedicationUiState> = _uiState.asStateFlow()
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            if (session.isLoggedIn) {
                loadMedications(session)
            } else {
                _uiState.value = MedicationUiState()
            }
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        super.onCleared()
    }

    fun updateSearchQuery(query: String) {
        _uiState.update { it.copy(searchQuery = query) }
    }

    fun clearMessages() {
        _uiState.update { it.copy(errorMessage = null, successMessage = null) }
    }

    fun loadMedications(session: AppSession = sessionManager.sessionState.value) {
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            when (val result = apiClient.getMedications(session.serverUrl, token, householdId)) {
                is ApiResult.Success -> {
                    _uiState.update { it.copy(isLoading = false, medications = result.data) }
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error loading medications") }
                }
            }
        }
    }

    fun createMedication(payload: CreateMedicationPayload, locationId: Int, onComplete: () -> Unit) {
        val session = sessionManager.sessionState.value
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            when (val result = apiClient.createMedication(session.serverUrl, token, householdId, locationId, payload)) {
                is ApiResult.Success -> {
                    _uiState.update { current ->
                        current.copy(
                            isLoading = false,
                            medications = current.medications + result.data,
                            successMessage = "Added ${result.data.name}"
                        )
                    }
                    onComplete()
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error creating medication") }
                }
            }
        }
    }

    fun recordStockRemoval(medicationId: Long, quantity: Double, reason: String, onComplete: () -> Unit) {
        val session = sessionManager.sessionState.value
        val token = session.accessToken ?: return
        val householdId = session.household?.id ?: return

        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true) }
            val payload = RecordStockRemovalPayload(
                medicationId = medicationId,
                quantity = quantity,
                reason = reason,
                removedAt = OffsetDateTime.now().toString()
            )
            when (val result = apiClient.recordStockRemoval(session.serverUrl, token, householdId, payload)) {
                is ApiResult.Success -> {
                    _uiState.update { current ->
                        current.copy(
                            isLoading = false,
                            medications = current.medications.map { if (it.id == medicationId) result.data else it },
                            successMessage = "Stock updated for ${result.data.name}"
                        )
                    }
                    onComplete()
                }
                is ApiResult.Error -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = result.message) }
                }
                is ApiResult.NetworkError -> {
                    _uiState.update { it.copy(isLoading = false, errorMessage = "Network error updating stock") }
                }
            }
        }
    }

    class Factory(private val sessionManager: SessionManager) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return MedicationViewModel(sessionManager) as T
        }
    }
}
