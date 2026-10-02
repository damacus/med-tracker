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
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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

@Composable
fun ScheduleFormScreen(
    people: List<PersonDto>,
    medications: List<MedicationDto>,
    onBackClick: () -> Unit,
    onSubmit: (CreateSchedulePayload) -> Unit,
    modifier: Modifier = Modifier
) {
    var selectedPersonIdText by remember { mutableStateOf(people.firstOrNull()?.id?.toString() ?: "1") }
    var selectedMedicationIdText by remember { mutableStateOf(medications.firstOrNull()?.id?.toString() ?: "1") }
    var doseAmountText by remember { mutableStateOf("1.0") }
    var doseUnit by remember { mutableStateOf("tablet") }
    var frequency by remember { mutableStateOf("daily") }
    var startDateText by remember { mutableStateOf(LocalDate.now().toString()) }

    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 16.dp, vertical = 12.dp)
                .verticalScroll(rememberScrollState())
        ) {
            OutlinedTextField(
                value = selectedPersonIdText,
                onValueChange = { selectedPersonIdText = it },
                label = { Text("Person ID") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )

            Spacer(modifier = Modifier.height(12.dp))

            OutlinedTextField(
                value = selectedMedicationIdText,
                onValueChange = { selectedMedicationIdText = it },
                label = { Text("Medication ID") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )

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
                    onClick = {
                        val personId = selectedPersonIdText.toLongOrNull() ?: 1L
                        val medId = selectedMedicationIdText.toLongOrNull() ?: 1L
                        val doseAmt = doseAmountText.toDoubleOrNull() ?: 1.0

                        val payload = CreateSchedulePayload(
                            personId = personId,
                            medicationId = medId,
                            doseAmount = doseAmt,
                            doseUnit = doseUnit.ifBlank { "tablet" },
                            frequency = frequency.ifBlank { "daily" },
                            startDate = startDateText
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
