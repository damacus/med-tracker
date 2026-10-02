package io.damacus.medtracker.ui.household

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

enum class HouseholdRole(val key: String, val label: String) {
    MEMBER("member", "Member / Carer"),
    ADMINISTRATOR("administrator", "Administrator")
}

@Composable
fun InvitationDialog(
    onDismiss: () -> Unit,
    onConfirmSend: (email: String, role: String) -> Unit
) {
    var email by remember { mutableStateOf("") }
    var selectedRole by remember { mutableStateOf(HouseholdRole.MEMBER) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(text = "Invite Household Member") },
        text = {
            Column {
                OutlinedTextField(
                    value = email,
                    onValueChange = { email = it },
                    label = { Text("Email Address") },
                    modifier = Modifier.fillMaxWidth()
                )

                Spacer(modifier = Modifier.height(12.dp))
                Text(text = "Select Role:", style = MaterialTheme.typography.labelLarge)
                Spacer(modifier = Modifier.height(4.dp))

                HouseholdRole.entries.forEach { role ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .selectable(
                                selected = (selectedRole == role),
                                onClick = { selectedRole = role }
                            )
                            .padding(vertical = 2.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        RadioButton(
                            selected = (selectedRole == role),
                            onClick = { selectedRole = role }
                        )
                        Text(
                            text = role.label,
                            modifier = Modifier.padding(start = 8.dp)
                        )
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                enabled = email.contains("@") && email.contains("."),
                onClick = { onConfirmSend(email.trim(), selectedRole.key) }
            ) {
                Text("Send Invitation")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel")
            }
        }
    )
}
