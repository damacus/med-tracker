package io.damacus.medtracker.ui.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.MedTrackerApi
import io.damacus.medtracker.data.api.NotificationPreferencesDto
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ProfileUiState(
    val sessionRevision: String? = null,
    val notificationPreferences: NotificationPreferencesDto? = null,
    val isLoading: Boolean = false,
    val isSaving: Boolean = false,
    val errorMessage: String? = null,
    val statusMessage: String? = null
) {
    fun forSession(session: AppSession): ProfileUiState =
        if (sessionRevision == session.revision) this else ProfileUiState(sessionRevision = session.revision)
}

class ProfileViewModel(
    private val sessionManager: SessionManager,
    private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
) : ViewModel() {

    private val _uiState = MutableStateFlow(ProfileUiState())
    val uiState: StateFlow<ProfileUiState> = _uiState.asStateFlow()
    private var sessionWork: Job = SupervisorJob(viewModelScope.coroutineContext[Job])
    private val sessionObservation: AutoCloseable

    init {
        sessionObservation = sessionManager.observeSession { session ->
            sessionWork.cancel()
            sessionWork = SupervisorJob(viewModelScope.coroutineContext[Job])
            _uiState.value = ProfileUiState(sessionRevision = session.revision)
            if (session.isLoggedIn && session.household != null) {
                loadNotificationPreferences(session)
            }
        }
    }

    override fun onCleared() {
        sessionObservation.close()
        sessionWork.cancel()
        super.onCleared()
    }

    fun loadNotificationPreferences(session: AppSession = sessionManager.sessionState.value) {
        val token = session.accessToken?.takeIf { it.isNotBlank() } ?: return
        val householdId = session.household?.id ?: return
        val work = sessionWork

        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) { it.copy(isLoading = true, errorMessage = null) }
            val result = apiClient.getNotificationPreferences(session.serverUrl, token, householdId)
            if (!isCurrentSession(session, work)) return@launch

            when (result) {
                is ApiResult.Success -> {
                    updateForSession(session, work) {
                        it.copy(
                            notificationPreferences = result.data,
                            isLoading = false,
                            errorMessage = null
                        )
                    }
                }
                is ApiResult.Error -> {
                    if (result.statusCode == 404 || result.code.equals("not_found", ignoreCase = true)) {
                        updateForSession(session, work) {
                            it.copy(
                                notificationPreferences = NotificationPreferencesDto(),
                                isLoading = false,
                                errorMessage = null
                            )
                        }
                    } else {
                        updateForSession(session, work) {
                            it.copy(
                                isLoading = false,
                                errorMessage = result.message
                            )
                        }
                    }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) {
                        it.copy(
                            isLoading = false,
                            errorMessage = "Unable to load notification preferences. Check your connection."
                        )
                    }
                }
            }
        }
    }

    fun updateNotificationPreferences(updated: NotificationPreferencesDto) {
        val session = sessionManager.sessionState.value
        val token = session.accessToken?.takeIf { it.isNotBlank() } ?: return
        val householdId = session.household?.id ?: return
        val work = sessionWork
        val previous = _uiState.value.notificationPreferences

        CoroutineScope(viewModelScope.coroutineContext + work).launch {
            if (!isCurrentSession(session, work)) return@launch
            updateForSession(session, work) {
                it.copy(
                    notificationPreferences = updated,
                    isSaving = true,
                    errorMessage = null,
                    statusMessage = null
                )
            }
            val result = apiClient.updateNotificationPreferences(
                baseUrl = session.serverUrl,
                accessToken = token,
                householdId = householdId,
                preferences = updated
            )
            if (!isCurrentSession(session, work)) return@launch

            when (result) {
                is ApiResult.Success -> {
                    updateForSession(session, work) {
                        it.copy(
                            notificationPreferences = result.data,
                            isSaving = false,
                            errorMessage = null,
                            statusMessage = "Notification preferences saved"
                        )
                    }
                }
                is ApiResult.Error -> {
                    updateForSession(session, work) {
                        it.copy(
                            notificationPreferences = previous,
                            isSaving = false,
                            errorMessage = result.message,
                            statusMessage = null
                        )
                    }
                }
                is ApiResult.NetworkError -> {
                    updateForSession(session, work) {
                        it.copy(
                            notificationPreferences = previous,
                            isSaving = false,
                            errorMessage = "Unable to save notification preferences. Check your connection.",
                            statusMessage = null
                        )
                    }
                }
            }
        }
    }

    private fun isCurrentSession(session: AppSession, work: Job): Boolean =
        work === sessionWork && sessionManager.sessionState.value.revision == session.revision

    private fun updateForSession(
        session: AppSession,
        work: Job,
        transform: (ProfileUiState) -> ProfileUiState
    ) {
        if (!isCurrentSession(session, work)) return
        _uiState.update { transform(it.forSession(session)) }
    }

    class Factory(
        private val sessionManager: SessionManager,
        private val apiClient: MedTrackerApi = GeneratedMedTrackerApi()
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return ProfileViewModel(sessionManager, apiClient) as T
        }
    }
}
