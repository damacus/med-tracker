package io.damacus.medtracker.ui.dose

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

enum class DoseOutcomeType {
    TAKEN,
    NOT_TAKEN,
    REOPEN
}

enum class NotTakenReason(val label: String) {
    MISSED("Missed / Forgotten"),
    REFUSED("Refused by Patient"),
    HELD("Held by Clinician"),
    UNAVAILABLE("Patient Unavailable"),
    OTHER("Other")
}

@Composable
fun DoseOutcomeDialog(
    medicationName: String,
    onDismiss: () -> Unit,
    onConfirmTaken: () -> Unit,
    onConfirmNotTaken: (reason: String, notes: String) -> Unit,
    onConfirmReopen: () -> Unit
) {
    var selectedOutcomeType by remember { mutableStateOf(DoseOutcomeType.TAKEN) }
    var selectedReason by remember { mutableStateOf(NotTakenReason.MISSED) }
    var notes by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(text = "Record Outcome: $medicationName") },
        text = {
            Column {
                Text(text = "Select outcome:", style = MaterialTheme.typography.labelLarge)
                Spacer(modifier = Modifier.height(8.dp))

                DoseOutcomeType.entries.forEach { outcomeType ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .selectable(
                                selected = (selectedOutcomeType == outcomeType),
                                onClick = { selectedOutcomeType = outcomeType }
                            )
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        RadioButton(
                            selected = (selectedOutcomeType == outcomeType),
                            onClick = { selectedOutcomeType = outcomeType }
                        )
                        Text(
                            text = when (outcomeType) {
                                DoseOutcomeType.TAKEN -> "Taken"
                                DoseOutcomeType.NOT_TAKEN -> "Not Taken"
                                DoseOutcomeType.REOPEN -> "Reopen / Correction"
                            },
                            modifier = Modifier.padding(start = 8.dp)
                        )
                    }
                }

                if (selectedOutcomeType == DoseOutcomeType.NOT_TAKEN) {
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(text = "Reason for not taking:", style = MaterialTheme.typography.labelLarge)
                    Spacer(modifier = Modifier.height(4.dp))

                    NotTakenReason.entries.forEach { reason ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .selectable(
                                    selected = (selectedReason == reason),
                                    onClick = { selectedReason = reason }
                                )
                                .padding(vertical = 2.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            RadioButton(
                                selected = (selectedReason == reason),
                                onClick = { selectedReason = reason }
                            )
                            Text(
                                text = reason.label,
                                modifier = Modifier.padding(start = 8.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = notes,
                        onValueChange = { notes = it },
                        label = { Text("Notes (optional)") },
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    when (selectedOutcomeType) {
                        DoseOutcomeType.TAKEN -> onConfirmTaken()
                        DoseOutcomeType.NOT_TAKEN -> onConfirmNotTaken(selectedReason.name.lowercase(), notes)
                        DoseOutcomeType.REOPEN -> onConfirmReopen()
                    }
                }
            ) {
                Text("Confirm")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}
