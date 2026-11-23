package io.damacus.medtracker.ui.household

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
import androidx.compose.material3.Checkbox
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.data.api.CreatePersonPayload
import io.damacus.medtracker.ui.theme.MedTrackerPrimary
import java.time.LocalDate
import java.time.format.DateTimeParseException

private fun isValidIsoDate(value: String): Boolean =
    try {
        LocalDate.parse(value.trim())
        true
    } catch (_: DateTimeParseException) {
        false
    }

@Composable
fun PersonFormDialog(
    onDismiss: () -> Unit,
    onSubmit: (name: String, email: String) -> Unit = { _, _ -> },
    onSubmitPerson: ((CreatePersonPayload) -> Unit)? = null
) {
    var nameText by remember { mutableStateOf("") }
    var dateOfBirthText by remember { mutableStateOf("") }
    var personType by remember { mutableStateOf("minor") }
    var guardianDeclaration by remember { mutableStateOf(false) }
    var emailText by remember { mutableStateOf("") }

    val dobValid = isValidIsoDate(dateOfBirthText)
    val canSubmit = nameText.isNotBlank() && dobValid && guardianDeclaration

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                text = "Add Dependent",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold
            )
        },
        text = {
            Column(modifier = Modifier.fillMaxWidth()) {
                OutlinedTextField(
                    value = nameText,
                    onValueChange = { nameText = it },
                    label = { Text("Full Name *") },
                    placeholder = { Text("e.g. Alex Smith") },
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    singleLine = true
                )

                Spacer(modifier = Modifier.height(10.dp))

                OutlinedTextField(
                    value = dateOfBirthText,
                    onValueChange = { dateOfBirthText = it },
                    label = { Text("Date of Birth (YYYY-MM-DD) *") },
                    placeholder = { Text("2018-05-14") },
                    isError = dateOfBirthText.isNotBlank() && !dobValid,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    singleLine = true
                )

                Spacer(modifier = Modifier.height(10.dp))

                Text(text = "Dependent Type", style = MaterialTheme.typography.labelLarge)
                listOf("minor" to "Minor", "dependent_adult" to "Dependent Adult").forEach { (value, label) ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .selectable(selected = personType == value, onClick = { personType = value })
                            .padding(vertical = 2.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        RadioButton(selected = personType == value, onClick = { personType = value })
                        Text(text = label, modifier = Modifier.padding(start = 6.dp))
                    }
                }

                Spacer(modifier = Modifier.height(8.dp))

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(
                        checked = guardianDeclaration,
                        onCheckedChange = { guardianDeclaration = it }
                    )
                    Text(
                        text = "I declare legal guardian authority for this dependent",
                        style = MaterialTheme.typography.bodySmall
                    )
                }

                Spacer(modifier = Modifier.height(8.dp))

                OutlinedTextField(
                    value = emailText,
                    onValueChange = { emailText = it },
                    label = { Text("Email Address (Optional)") },
                    placeholder = { Text("alex@example.com") },
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    singleLine = true
                )
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    val payload = CreatePersonPayload(
                        name = nameText.trim(),
                        dateOfBirth = dateOfBirthText.trim(),
                        personType = personType,
                        initialGuardianDeclaration = guardianDeclaration,
                        email = emailText.trim().ifBlank { null }
                    )
                    if (onSubmitPerson != null) {
                        onSubmitPerson(payload)
                    } else {
                        onSubmit(nameText.trim(), emailText.trim())
                    }
                },
                enabled = canSubmit,
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(containerColor = MedTrackerPrimary)
            ) {
                Text("Add Dependent")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}
