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
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
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
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.LocationDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary

val MEDICATION_CATEGORIES: List<String> = listOf(
    "Analgesic",
    "Antibiotic",
    "Anticoagulant",
    "Anticonvulsant",
    "Antidepressant",
    "Antidiabetic",
    "Antiemetic",
    "Antifungal",
    "Antihistamine",
    "Antihypertensive",
    "Anti-Inflammatory",
    "Antiparasitic",
    "Antipsychotic",
    "Antiviral",
    "Anxiolytic",
    "Cardiovascular",
    "Cholesterol",
    "Contraceptive",
    "Dermatological",
    "Gastrointestinal",
    "Hormonal",
    "Immunosuppressant",
    "Migraine",
    "Mineral",
    "Muscle Relaxant",
    "Neurological",
    "Oncology",
    "Ophthalmic",
    "Osmotic Laxative",
    "Opioid",
    "Osteoporosis",
    "Respiratory",
    "Sleep Aid",
    "Smoking Cessation",
    "Supplement",
    "Thyroid",
    "Urological",
    "Vitamin",
    "Weight Management"
)

val MEDICATION_DOSE_UNITS: List<String> = listOf(
    "tablet",
    "capsule",
    "gummy",
    "mg",
    "ml",
    "g",
    "mcg",
    "IU",
    "spray",
    "drop",
    "sachet",
    "pad"
)

fun defaultMedicationLocationId(locations: List<LocationDto>): Int? =
    locations.singleOrNull()?.id?.toInt()

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MedicationFormScreen(
    onBackClick: () -> Unit,
    onSubmit: (CreateMedicationPayload, locationId: Int) -> Unit,
    modifier: Modifier = Modifier,
    locations: List<LocationDto> = emptyList()
) {
    var name by remember { mutableStateOf("") }
    var selectedCategory by remember { mutableStateOf<String?>(null) }
    var categoryExpanded by remember { mutableStateOf(false) }
    var doseAmountText by remember { mutableStateOf("") }
    var doseUnit by remember { mutableStateOf("tablet") }
    var doseUnitExpanded by remember { mutableStateOf(false) }
    var supplyText by remember { mutableStateOf("") }
    var reorderText by remember { mutableStateOf("") }
    var selectedLocationId by remember { mutableStateOf(defaultMedicationLocationId(locations)) }
    var locationExpanded by remember { mutableStateOf(false) }

    LaunchedEffect(locations) {
        if (selectedLocationId == null || locations.none { it.id?.toInt() == selectedLocationId }) {
            selectedLocationId = defaultMedicationLocationId(locations)
        }
    }

    val selectedLocationName = locations.firstOrNull { it.id?.toInt() == selectedLocationId }?.name ?: ""
    val canSubmit = name.isNotBlank() && selectedLocationId != null

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

            ExposedDropdownMenuBox(
                expanded = locationExpanded,
                onExpandedChange = { if (locations.isNotEmpty()) locationExpanded = it }
            ) {
                OutlinedTextField(
                    value = selectedLocationName,
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Storage Location *") },
                    placeholder = {
                        Text(if (locations.isEmpty()) "No household locations available" else "Select a location")
                    },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = locationExpanded) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .menuAnchor(MenuAnchorType.PrimaryNotEditable),
                    shape = RoundedCornerShape(12.dp)
                )
                ExposedDropdownMenu(
                    expanded = locationExpanded,
                    onDismissRequest = { locationExpanded = false }
                ) {
                    locations.forEach { location ->
                        val locId = location.id?.toInt() ?: return@forEach
                        DropdownMenuItem(
                            text = { Text(location.name) },
                            onClick = {
                                selectedLocationId = locId
                                locationExpanded = false
                            }
                        )
                    }
                }
            }

            if (locations.isEmpty()) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "Add a household location before creating a medication.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            ExposedDropdownMenuBox(
                expanded = categoryExpanded,
                onExpandedChange = { categoryExpanded = it }
            ) {
                OutlinedTextField(
                    value = selectedCategory ?: "",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Category") },
                    placeholder = { Text("Select a category (optional)") },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = categoryExpanded) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .menuAnchor(MenuAnchorType.PrimaryNotEditable),
                    shape = RoundedCornerShape(12.dp)
                )
                ExposedDropdownMenu(
                    expanded = categoryExpanded,
                    onDismissRequest = { categoryExpanded = false }
                ) {
                    DropdownMenuItem(
                        text = { Text("None") },
                        onClick = {
                            selectedCategory = null
                            categoryExpanded = false
                        }
                    )
                    MEDICATION_CATEGORIES.forEach { category ->
                        DropdownMenuItem(
                            text = { Text(category) },
                            onClick = {
                                selectedCategory = category
                                categoryExpanded = false
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

                ExposedDropdownMenuBox(
                    expanded = doseUnitExpanded,
                    onExpandedChange = { doseUnitExpanded = it },
                    modifier = Modifier.weight(1f)
                ) {
                    OutlinedTextField(
                        value = doseUnit,
                        onValueChange = {},
                        readOnly = true,
                        label = { Text("Dose Unit") },
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = doseUnitExpanded) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .menuAnchor(MenuAnchorType.PrimaryNotEditable),
                        shape = RoundedCornerShape(12.dp)
                    )
                    ExposedDropdownMenu(
                        expanded = doseUnitExpanded,
                        onDismissRequest = { doseUnitExpanded = false }
                    ) {
                        MEDICATION_DOSE_UNITS.forEach { unit ->
                            DropdownMenuItem(
                                text = { Text(unit) },
                                onClick = {
                                    doseUnit = unit
                                    doseUnitExpanded = false
                                }
                            )
                        }
                    }
                }
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
                        val locId = selectedLocationId ?: return@Button
                        val payload = CreateMedicationPayload(
                            name = name.trim(),
                            category = selectedCategory,
                            doseAmount = doseAmountText.toDoubleOrNull(),
                            doseUnit = doseUnit.ifBlank { null },
                            currentSupply = supplyText.toDoubleOrNull(),
                            reorderThreshold = reorderText.toDoubleOrNull()
                        )
                        onSubmit(payload, locId)
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
