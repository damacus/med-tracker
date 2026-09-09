package io.damacus.medtracker.ui.dose

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

enum class StockRemovalReason(val label: String) {
    DAMAGED("Damaged"),
    DROPPED("Dropped / Spilled"),
    EXPIRED("Expired"),
    DISCARDED("Discarded"),
    LOST("Lost"),
    OTHER("Other")
}

@Composable
fun StockRemovalDialog(
    medicationName: String,
    onDismiss: () -> Unit,
    onConfirmRemoval: (quantity: Double, reason: String) -> Unit
) {
    var quantityText by remember { mutableStateOf("1") }
    var selectedReason by remember { mutableStateOf(StockRemovalReason.DAMAGED) }
    var isError by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(text = "Record Stock Loss: $medicationName") },
        text = {
            Column {
                OutlinedTextField(
                    value = quantityText,
                    onValueChange = {
                        quantityText = it
                        isError = (it.toDoubleOrNull() == null || (it.toDoubleOrNull() ?: 0.0) <= 0)
                    },
                    label = { Text("Quantity Removed") },
                    isError = isError,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth()
                )

                Spacer(modifier = Modifier.height(12.dp))
                Text(text = "Reason for removal:", style = MaterialTheme.typography.labelLarge)
                Spacer(modifier = Modifier.height(4.dp))

                StockRemovalReason.entries.forEach { reason ->
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
            }
        },
        confirmButton = {
            TextButton(
                enabled = !isError && (quantityText.toDoubleOrNull() ?: 0.0) > 0,
                onClick = {
                    val qty = quantityText.toDoubleOrNull() ?: 1.0
                    onConfirmRemoval(qty, selectedReason.name.lowercase())
                }
            ) {
                Text("Confirm Removal")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}
