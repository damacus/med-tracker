package io.damacus.medtracker.ui.healthevents

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary
import java.time.LocalDate
import java.time.format.DateTimeParseException

private fun isValidEventDate(value: String): Boolean =
    try {
        LocalDate.parse(value.trim())
        true
    } catch (_: DateTimeParseException) {
        false
    }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HealthEventFormDialog(
    people: List<PersonDto>,
    onDismiss: () -> Unit,
    onSubmit: (personId: Long, title: String, notes: String) -> Unit,
    onSubmitDetailed: ((personId: Long, title: String, notes: String, eventKind: String, startedOn: String) -> Unit)? = null
) {
    var selectedPersonId by remember { mutableStateOf(people.firstOrNull()?.id) }
    var personExpanded by remember { mutableStateOf(false) }
    var selectedEventKind by remember { mutableStateOf("suspected_side_effect") }
    var startedOnText by remember { mutableStateOf(LocalDate.now().toString()) }
    var titleText by remember { mutableStateOf("") }
    var notesText by remember { mutableStateOf("") }

    val selectedPersonName = people.firstOrNull { it.id == selectedPersonId }?.name ?: ""
    val dateValid = isValidEventDate(startedOnText)
    val canSubmit = selectedPersonId != null && titleText.isNotBlank() && dateValid

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                text = "Log Health Event",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold
            )
        },
        text = {
            Column(modifier = Modifier.fillMaxWidth()) {
                ExposedDropdownMenuBox(
                    expanded = personExpanded,
                    onExpandedChange = { if (people.isNotEmpty()) personExpanded = it }
                ) {
                    OutlinedTextField(
                        value = selectedPersonName,
                        onValueChange = {},
                        readOnly = true,
                        label = { Text("Person *") },
                        placeholder = {
                            Text(if (people.isEmpty()) "No people available" else "Select a person")
                        },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = personExpanded) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .menuAnchor(MenuAnchorType.PrimaryNotEditable),
                        shape = RoundedCornerShape(12.dp)
                    )
                    ExposedDropdownMenu(
                        expanded = personExpanded,
                        onDismissRequest = { personExpanded = false }
                    ) {
                        people.forEach { person ->
                            val id = person.id ?: return@forEach
                            DropdownMenuItem(
                                text = { Text(person.name) },
                                onClick = {
                                    selectedPersonId = id
                                    personExpanded = false
                                }
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                Text(text = "Event Kind", style = MaterialTheme.typography.labelLarge)
                listOf(
                    "suspected_side_effect" to "Suspected Side Effect",
                    "illness" to "Illness / Symptom"
                ).forEach { (kind, label) ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .selectable(selected = selectedEventKind == kind, onClick = { selectedEventKind = kind })
                            .padding(vertical = 2.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        RadioButton(selected = selectedEventKind == kind, onClick = { selectedEventKind = kind })
                        Text(text = label, modifier = Modifier.padding(start = 6.dp))
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                OutlinedTextField(
                    value = startedOnText,
                    onValueChange = { startedOnText = it },
                    label = { Text("Date (YYYY-MM-DD) *") },
                    isError = startedOnText.isNotBlank() && !dateValid,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    singleLine = true
                )

                Spacer(modifier = Modifier.height(10.dp))

                OutlinedTextField(
                    value = titleText,
                    onValueChange = { titleText = it },
                    label = { Text("Title / Side Effect Name *") },
                    placeholder = { Text("e.g. Mild Dizziness, Rash, Nausea") },
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp)
                )

                Spacer(modifier = Modifier.height(10.dp))

                OutlinedTextField(
                    value = notesText,
                    onValueChange = { notesText = it },
                    label = { Text("Notes / Observations") },
                    placeholder = { Text("Observed 30 mins after dose...") },
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    minLines = 2
                )
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    val personId = selectedPersonId ?: return@Button
                    if (onSubmitDetailed != null) {
                        onSubmitDetailed(personId, titleText.trim(), notesText.trim(), selectedEventKind, startedOnText.trim())
                    } else {
                        onSubmit(personId, titleText.trim(), notesText.trim())
                    }
                },
                enabled = canSubmit,
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(containerColor = MedTrackerPrimary)
            ) {
                Text("Log Event")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}
