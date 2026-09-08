package io.damacus.medtracker.ui.dashboard

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.data.model.*

@Composable
fun MedicationPauseControls(state: MedicationPauseState, personId: Long?, controller: MedicationPauseController, displayedScheduleIds: Set<String> = emptySet()) {
    if (!state.supported) return
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Treatment pauses", style = MaterialTheme.typography.titleLarge)
        Text("Pause and resume need an internet connection. Changes start when the server confirms them.")
        state.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        state.additionalActiveSources(personId, displayedScheduleIds).forEach { source ->
            Card(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(12.dp)) {
                    Text(source.name, style = MaterialTheme.typography.titleMedium)
                    Text(source.personName)
                    Row {
                        TextButton(onClick = { controller.edit(source) }) { Text("Pause") }
                        TextButton(onClick = { controller.showHistory(source) }) { Text("Pause history") }
                    }
                }
            }
        }
        val paused = state.pausedSources(personId)
        if (paused.isNotEmpty()) {
            Text("Paused treatments (${paused.size})", style = MaterialTheme.typography.titleMedium)
            paused.forEach { source ->
                Card(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(12.dp)) {
                        Text(source.name, style = MaterialTheme.typography.titleMedium)
                        Text(source.personName)
                        PausedTreatmentDetails(source)
                        Row {
                            TextButton(onClick = { controller.resume(source) }, enabled = source.currentPauseId != null && state.busySource == null) {
                                Text(if (state.busySource == source.key) "Resuming…" else "Resume")
                            }
                            TextButton(onClick = { controller.showHistory(source) }) { Text("Pause history") }
                        }
                    }
                }
            }
        }
    }
    state.editing?.let { source ->
        PauseEditor(source, state.form, controller::reason, controller::note, controller::submit, controller::dismiss)
    }
    state.historySource?.let { source ->
        AlertDialog(
            onDismissRequest = controller::dismiss,
            title = { Text("Pause history: ${source.name}") },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    if (state.loadingHistory) CircularProgressIndicator()
                    state.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                    if (!state.loadingHistory && state.error == null && state.history.isEmpty()) Text("No pauses recorded.")
                    state.orderedHistory.forEach { period ->
                        Column {
                            Text(period.reasonLabel, style = MaterialTheme.typography.titleSmall)
                            period.note?.takeIf { it.isNotBlank() }?.let { Text(it) }
                            Text("Started: ${period.startLabel}")
                            Text("Recorded by: ${period.recordedByLabel}")
                            Text("Ended: ${period.endedAt?.let(::formatPauseDate) ?: "Still paused"}")
                            if (period.endedAt != null) Text("Resumed by: ${period.resumedBy ?: "Not recorded"}")
                            if (period.legacyContext) Text("Historical pause; some details were not recorded.")
                        }
                    }
                }
            },
            confirmButton = { TextButton(onClick = controller::dismiss) { Text("Close") } }
        )
    }
}

@Composable
fun PausedTreatmentDetails(source: PauseSource) {
    Column {
        Text(PauseReason.entries.find { it.value == source.currentReason }?.label ?: "Reason not recorded")
        source.currentNote?.takeIf { it.isNotBlank() }?.let { Text(it) }
    }
}

@Composable
fun SchedulePauseActions(controller: MedicationPauseController, portableId: String?) {
    val state by controller.state.collectAsState()
    if (!state.supported) return
    val source = state.sources.find { it.type == "schedule" && it.id == portableId && !it.paused } ?: return
    Row {
        TextButton(onClick = { controller.edit(source) }) { Text("Pause") }
        TextButton(onClick = { controller.showHistory(source) }) { Text("Pause history") }
    }
}

@Composable
fun PauseEditor(source: PauseSource, form: PauseForm, onReason: (PauseReason) -> Unit, onNote: (String) -> Unit, onSubmit: () -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = { if (!form.submitting) onDismiss() },
        title = { Text("Pause ${source.name}") },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                Text("Choose a reason. The pause starts when the server confirms it.")
                Column(Modifier.selectableGroup()) {
                    PauseReason.entries.forEach { reason ->
                        Row(Modifier.fillMaxWidth().selectable(selected = form.reason == reason, enabled = !form.submitting, role = Role.RadioButton, onClick = { onReason(reason) }).padding(vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                            RadioButton(selected = form.reason == reason, onClick = null, enabled = !form.submitting)
                            Text(reason.label, Modifier.padding(start = 8.dp))
                        }
                    }
                }
                OutlinedTextField(value = form.note, onValueChange = onNote, label = { Text("Note (optional)") }, enabled = !form.submitting)
                form.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(onClick = onSubmit, enabled = form.canSubmit) { Text(if (form.submitting) "Pausing…" else "Confirm pause") } },
        dismissButton = { TextButton(onClick = onDismiss, enabled = !form.submitting) { Text("Cancel") } }
    )
}
