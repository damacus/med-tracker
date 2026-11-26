package io.damacus.medtracker.ui.schedule

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenu
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary
import java.time.LocalDate
import java.time.format.DateTimeParseException

fun isValidScheduleStartDate(value: String): Boolean =
    try {
        LocalDate.parse(value.trim())
        true
    } catch (_: DateTimeParseException) {
        false
    }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ScheduleFormScreen(
    people: List<PersonDto>,
    medications: List<MedicationDto>,
    onBackClick: () -> Unit,
    onSubmit: (CreateSchedulePayload) -> Unit,
    modifier: Modifier = Modifier
) {
    var selectedPersonId by remember { mutableStateOf(people.firstOrNull()?.id) }
    var personExpanded by remember { mutableStateOf(false) }
    var selectedMedicationId by remember { mutableStateOf(medications.firstOrNull()?.id) }
    var medicationExpanded by remember { mutableStateOf(false) }
    var doseAmountText by remember { mutableStateOf("1.0") }
    var doseUnit by remember { mutableStateOf("tablet") }
    var frequency by remember { mutableStateOf("daily") }
    var startDateText by remember { mutableStateOf(LocalDate.now().toString()) }

    LaunchedEffect(people) {
        if (selectedPersonId == null || people.none { it.id == selectedPersonId }) {
            selectedPersonId = people.firstOrNull()?.id
        }
    }

    LaunchedEffect(medications) {
        if (selectedMedicationId == null || medications.none { it.id == selectedMedicationId }) {
            selectedMedicationId = medications.firstOrNull()?.id
        }
    }

    val selectedPersonName = people.firstOrNull { it.id == selectedPersonId }?.name ?: ""
    val selectedMedication = medications.firstOrNull { it.id == selectedMedicationId }
    val selectedMedicationName = selectedMedication?.displayName ?: selectedMedication?.name ?: ""
    val startDateValid = isValidScheduleStartDate(startDateText)
    val doseAmount = doseAmountText.toDoubleOrNull()
    val canSubmit = selectedPersonId != null &&
        selectedMedicationId != null &&
        doseAmount != null &&
        doseAmount > 0.0 &&
        startDateValid

    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 16.dp, vertical = 12.dp)
                .verticalScroll(rememberScrollState())
        ) {
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
                        Text(if (people.isEmpty()) "No household people available" else "Select a person")
                    },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = personExpanded) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
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

            Spacer(modifier = Modifier.height(12.dp))

            ExposedDropdownMenuBox(
                expanded = medicationExpanded,
                onExpandedChange = { if (medications.isNotEmpty()) medicationExpanded = it }
            ) {
                OutlinedTextField(
                    value = selectedMedicationName,
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Medication *") },
                    placeholder = {
                        Text(if (medications.isEmpty()) "No household medications available" else "Select a medication")
                    },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = medicationExpanded) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
                    shape = RoundedCornerShape(12.dp)
                )
                ExposedDropdownMenu(
                    expanded = medicationExpanded,
                    onDismissRequest = { medicationExpanded = false }
                ) {
                    medications.forEach { medication ->
                        val id = medication.id ?: return@forEach
                        DropdownMenuItem(
                            text = { Text(medication.displayName ?: medication.name) },
                            onClick = {
                                selectedMedicationId = id
                                medication.doseUnit?.takeIf { it.isNotBlank() }?.let { doseUnit = it }
                                medicationExpanded = false
                            }
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            Row(modifier = Modifier.fillMaxWidth()) {
                OutlinedTextField(
                    value = doseAmountText,
                    onValueChange = { doseAmountText = it },
                    label = { Text("Dose Amount") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(12.dp)
                )

                Spacer(modifier = Modifier.width(8.dp))

                OutlinedTextField(
                    value = doseUnit,
                    onValueChange = { doseUnit = it },
                    label = { Text("Dose Unit") },
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(12.dp)
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            OutlinedTextField(
                value = frequency,
                onValueChange = { frequency = it },
                label = { Text("Frequency (daily, twice_daily, weekly)") },
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )

            Spacer(modifier = Modifier.height(12.dp))

            OutlinedTextField(
                value = startDateText,
                onValueChange = { startDateText = it },
                label = { Text("Start Date (YYYY-MM-DD)") },
                isError = startDateText.isNotBlank() && !startDateValid,
                supportingText = if (startDateText.isNotBlank() && !startDateValid) {
                    { Text("Use YYYY-MM-DD format", color = MaterialTheme.colorScheme.error) }
                } else {
                    null
                },
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )

            Spacer(modifier = Modifier.height(24.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                OutlinedButton(
                    onClick = onBackClick,
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Cancel")
                }

                Button(
                    enabled = canSubmit,
                    onClick = {
                        val personId = selectedPersonId ?: return@Button
                        val medId = selectedMedicationId ?: return@Button
                        val doseAmt = doseAmount ?: return@Button

                        val payload = CreateSchedulePayload(
                            personId = personId,
                            medicationId = medId,
                            doseAmount = doseAmt,
                            doseUnit = doseUnit.ifBlank { "tablet" },
                            frequency = frequency.ifBlank { "daily" },
                            startDate = startDateText.trim()
                        )
                        onSubmit(payload)
                    },
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MedTrackerPrimary),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Create Schedule", fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}
