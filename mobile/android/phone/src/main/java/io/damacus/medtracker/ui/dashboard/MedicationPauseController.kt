package io.damacus.medtracker.ui.dashboard

import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.MedicationPauseGateway
import io.damacus.medtracker.data.model.*
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

class MedicationPauseController(
    private val session: AppSession,
    private val gateway: MedicationPauseGateway,
    private val scope: CoroutineScope,
    private val isCurrent: () -> Boolean,
    private val onConfirmed: () -> Unit
) {
    private val mutable = MutableStateFlow(MedicationPauseState())
    val state = mutable.asStateFlow()
    private var sourceGeneration = 0

    fun refresh() {
        if (state.value.form.submitting || state.value.busySource != null) return
        val generation = ++sourceGeneration
        scope.launch {
            val capability = gateway.supported(session)
            if (!isCurrent() || generation != sourceGeneration) return@launch
            if (capability is ApiResult.Success && !capability.data) {
                mutable.value = MedicationPauseState()
                return@launch
            }
            if (capability !is ApiResult.Success) {
                update { it.copy(error = message(capability)) }
                return@launch
            }
            val result = gateway.sources(session)
            if (generation != sourceGeneration) return@launch
            when (result) {
                is ApiResult.Success -> update { it.copy(supported = true, sources = result.data, error = null) }
                else -> update { it.copy(supported = true, error = message(result)) }
            }
        }
    }

    fun edit(source: PauseSource) {
        if (!isCurrent() || !state.value.supported || state.value.busySource != null || state.value.form.submitting || source !in state.value.sources || source.paused) return
        update { it.copy(editing = source, form = PauseForm()) }
    }

    fun reason(reason: PauseReason) = update { if (it.form.submitting) it else it.copy(form = it.form.copy(reason = reason)) }
    fun note(note: String) = update { if (it.form.submitting) it else it.copy(form = it.form.copy(note = note)) }
    fun dismiss() = update { if (it.form.submitting || it.busySource != null) it else it.copy(editing = null, historySource = null, error = null) }

    fun submit() {
        val current = state.value
        val source = current.editing ?: return
        val reason = current.form.reason ?: return
        if (!isCurrent() || !current.supported || current.busySource != null || !current.form.canSubmit) return
        sourceGeneration++
        update { it.copy(form = it.form.copy(submitting = true, error = null)) }
        scope.launch {
            when (val result = gateway.pause(session, source, reason, current.form.note, current.form.requestId)) {
                is ApiResult.Success -> {
                    if (!isCurrent()) return@launch
                    when (val sources = gateway.sources(session)) {
                        is ApiResult.Success -> update { it.copy(editing = null, form = PauseForm(), sources = sources.data) }
                        else -> update { it.copy(editing = null, form = PauseForm(), error = "Pause was accepted, but current treatment state could not be refreshed. Refresh before trying again.") }
                    }
                    if (isCurrent()) onConfirmed()
                }
                else -> update { it.copy(form = it.form.copy(submitting = false, error = message(result))) }
            }
        }
    }

    fun showHistory(source: PauseSource) {
        if (!isCurrent() || !state.value.supported || source !in state.value.sources) return
        update { it.copy(historySource = source, history = emptyList(), loadingHistory = true, error = null) }
        scope.launch {
            val result = gateway.history(session, source)
            if (state.value.historySource?.key != source.key) return@launch
            when (result) {
                is ApiResult.Success -> update { it.copy(history = result.data, loadingHistory = false) }
                else -> update { it.copy(error = message(result), loadingHistory = false) }
            }
        }
    }

    fun resume(source: PauseSource) {
        val pauseId = source.currentPauseId ?: return
        if (!isCurrent() || !source.paused || !state.value.supported || state.value.form.submitting || state.value.busySource != null || source !in state.value.sources) return
        sourceGeneration++
        update { it.copy(busySource = source.key, error = null) }
        scope.launch {
            when (val result = gateway.resume(session, pauseId)) {
                is ApiResult.Success -> {
                    if (!isCurrent()) return@launch
                    when (val sources = gateway.sources(session)) {
                        is ApiResult.Success -> update { it.copy(busySource = null, sources = sources.data) }
                        else -> update { it.copy(busySource = null, error = "Resume was accepted, but current treatment state could not be refreshed. Refresh before trying again.") }
                    }
                    if (isCurrent()) onConfirmed()
                }
                else -> update { it.copy(busySource = null, error = message(result)) }
            }
        }
    }

    private fun update(block: (MedicationPauseState) -> MedicationPauseState) {
        if (isCurrent()) mutable.update(block)
    }

    private fun message(result: ApiResult<*>): String = when (result) {
        is ApiResult.Error -> result.message
        is ApiResult.NetworkError -> "Connect to the internet and try again. No change was confirmed."
        is ApiResult.Success -> ""
    }
}
