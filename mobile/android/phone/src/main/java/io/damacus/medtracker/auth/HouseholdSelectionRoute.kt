package io.damacus.medtracker.auth

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.data.model.HouseholdSelection

@Composable
fun HouseholdSelectionRoute(
    selection: HouseholdSelection,
    isLoading: Boolean,
    errorMessage: String?,
    onRetry: () -> Unit = {},
    onLogout: () -> Unit = {},
    onSelect: (Long) -> Unit
) {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp, Alignment.CenterVertically)
    ) {
        Text("Choose a household")
        Text(if (selection.households.isEmpty()) "You are signed in, but have no available households." else "Choose which household to view. Your login works across your households.")
        errorMessage?.let { Text(it) }
        selection.households.forEach { household ->
            Button(
                onClick = { onSelect(household.id) },
                enabled = !isLoading,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(household.name)
            }
        }
        Button(onClick = onRetry, enabled = !isLoading) { Text("Reload households") }
        Button(onClick = onLogout, enabled = !isLoading) { Text("Sign out") }
    }
}
