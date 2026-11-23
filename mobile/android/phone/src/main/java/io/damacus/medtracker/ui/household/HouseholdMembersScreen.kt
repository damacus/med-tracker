package io.damacus.medtracker.ui.household

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material3.Badge
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.data.api.CreatePersonPayload
import io.damacus.medtracker.data.model.HouseholdInvitationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary

@Composable
fun HouseholdMembersScreen(
    people: List<PersonDto>,
    invitations: List<HouseholdInvitationDto>,
    isLoading: Boolean,
    onInviteClick: () -> Unit,
    modifier: Modifier = Modifier,
    onCreatePerson: (CreatePersonPayload) -> Unit = {},
    errorMessage: String? = null,
    successMessage: String? = null
) {
    var showAddDependentDialog by remember { mutableStateOf(false) }

    if (showAddDependentDialog) {
        PersonFormDialog(
            onDismiss = { showAddDependentDialog = false },
            onSubmitPerson = { payload ->
                showAddDependentDialog = false
                onCreatePerson(payload)
            }
        )
    }

    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 16.dp, vertical = 12.dp)
        ) {
            errorMessage?.let { error ->
                Surface(
                    color = MaterialTheme.colorScheme.errorContainer,
                    shape = MaterialTheme.shapes.medium,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = error,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(12.dp)
                    )
                }
                Spacer(modifier = Modifier.height(8.dp))
            }

            successMessage?.let { success ->
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    shape = MaterialTheme.shapes.medium,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = success,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(12.dp)
                    )
                }
                Spacer(modifier = Modifier.height(8.dp))
            }

            OutlinedButton(
                onClick = { showAddDependentDialog = true },
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("Add Dependent")
            }

            Spacer(modifier = Modifier.height(12.dp))

            if (isLoading && people.isEmpty() && invitations.isEmpty()) {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(color = MedTrackerPrimary)
                }
            } else {
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    contentPadding = PaddingValues(bottom = 80.dp)
                ) {
                    items(people, key = { it.id ?: it.name }) { person ->
                        PersonCard(person = person)
                    }

                    if (invitations.isNotEmpty()) {
                        item {
                            Spacer(modifier = Modifier.height(16.dp))
                            Text(
                                text = "Invitations",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold
                            )
                            Spacer(modifier = Modifier.height(8.dp))
                        }

                        items(invitations, key = { it.id ?: it.email }) { invitation ->
                            InvitationCard(invitation = invitation)
                        }
                    }
                }
            }
        }

        FloatingActionButton(
            onClick = onInviteClick,
            containerColor = MedTrackerPrimary,
            contentColor = MaterialTheme.colorScheme.onPrimary,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(16.dp)
        ) {
            Icon(Icons.Default.PersonAdd, contentDescription = "Invite Member")
        }
    }
}

@Composable
fun PersonCard(person: PersonDto) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
        Row(
            modifier = Modifier
                .padding(16.dp)
                .fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text(
                    text = person.name,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold
                )
                if (!person.email.isNullOrBlank()) {
                    Text(
                        text = person.email,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            if (!person.personType.isNullOrBlank()) {
                Badge(containerColor = MaterialTheme.colorScheme.secondaryContainer) {
                    Text(person.personType, color = MaterialTheme.colorScheme.onSecondaryContainer)
                }
            }
        }
    }
}

@Composable
fun InvitationCard(invitation: HouseholdInvitationDto) {
    val statusText = (invitation.status ?: "pending").replaceFirstChar { it.uppercase() }
    val badgeBg = when (invitation.status) {
        "accepted" -> MaterialTheme.colorScheme.primaryContainer
        "revoked", "expired" -> MaterialTheme.colorScheme.errorContainer
        else -> Color(0xFFFFF3CD)
    }
    val badgeFg = when (invitation.status) {
        "accepted" -> MaterialTheme.colorScheme.onPrimaryContainer
        "revoked", "expired" -> MaterialTheme.colorScheme.onErrorContainer
        else -> Color(0xFF856404)
    }
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp)
    ) {
        Row(
            modifier = Modifier
                .padding(12.dp)
                .fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text(
                    text = invitation.email,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = "Role: ${invitation.role}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Badge(containerColor = badgeBg) {
                Text(statusText, color = badgeFg)
            }
        }
    }
}
