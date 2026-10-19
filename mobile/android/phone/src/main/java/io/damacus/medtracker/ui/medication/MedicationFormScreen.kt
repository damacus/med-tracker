package io.damacus.medtracker.ui.medication

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
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
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
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.LocationDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary

@Composable
fun MedicationFormScreen(
    onBackClick: () -> Unit,
    onSubmit: (CreateMedicationPayload, locationId: Int?) -> Unit,
    locations: List<LocationDto> = emptyList(),
    sessionRevision: String = "",
    modifier: Modifier = Modifier
) {
    var name by remember(sessionRevision) { mutableStateOf("") }
    var category by remember(sessionRevision) { mutableStateOf("") }
    var doseAmountText by remember(sessionRevision) { mutableStateOf("") }
    var doseUnit by remember(sessionRevision) { mutableStateOf("tablet") }
    var supplyText by remember(sessionRevision) { mutableStateOf("") }
    var reorderText by remember(sessionRevision) { mutableStateOf("") }
    var selectedLocationId by remember(sessionRevision) { mutableStateOf<Int?>(null) }
    var labelMenuExpanded by remember(sessionRevision) { mutableStateOf(false) }
    val availableLocations = locations.filter { it.id != null && it.id in 1..Int.MAX_VALUE.toLong() }
    val selectedLocation = availableLocations.find { it.id?.toInt() == selectedLocationId }

    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 16.dp, vertical = 12.dp)
                .verticalScroll(rememberScrollState())
        ) {
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                label = { Text("Medication Name *") },
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )

            Spacer(modifier = Modifier.height(12.dp))

            OutlinedTextField(
                value = category,
                onValueChange = { category = it },
                label = { Text("Category (e.g. Analgesic, Antibiotic)") },
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
                    label = { Text("Dose Unit (mg, ml, tablet)") },
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(12.dp)
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            Row(modifier = Modifier.fillMaxWidth()) {
                OutlinedTextField(
                    value = supplyText,
                    onValueChange = { supplyText = it },
                    label = { Text("Current Supply") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(12.dp)
                )

                Spacer(modifier = Modifier.width(8.dp))

                OutlinedTextField(
                    value = reorderText,
                    onValueChange = { reorderText = it },
                    label = { Text("Reorder Threshold") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.weight(1f),
                    shape = RoundedCornerShape(12.dp)
                )
            }

            Spacer(modifier = Modifier.height(24.dp))

            Box {
                OutlinedButton(onClick = { labelMenuExpanded = true }) {
                    Text(selectedLocation?.name ?: "No storage label")
                }
                DropdownMenu(expanded = labelMenuExpanded, onDismissRequest = { labelMenuExpanded = false }) {
                    DropdownMenuItem(text = { Text("No storage label") }, onClick = {
                        selectedLocationId = null
                        labelMenuExpanded = false
                    })
                    availableLocations.forEach { location ->
                        DropdownMenuItem(text = { Text(location.name) }, onClick = {
                            selectedLocationId = location.id?.toInt()
                            labelMenuExpanded = false
                        })
                    }
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

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
                    enabled = name.isNotBlank(),
                    onClick = {
                        val payload = CreateMedicationPayload(
                            name = name.trim(),
                            category = category.ifBlank { null },
                            doseAmount = doseAmountText.toDoubleOrNull(),
                            doseUnit = doseUnit.ifBlank { null },
                            currentSupply = supplyText.toDoubleOrNull(),
                            reorderThreshold = reorderText.toDoubleOrNull()
                        )
                        onSubmit(payload, selectedLocation?.id?.toInt())
                    },
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MedTrackerPrimary),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Save Medication", fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}
