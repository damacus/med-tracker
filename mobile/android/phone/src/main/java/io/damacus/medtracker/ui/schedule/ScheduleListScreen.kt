package io.damacus.medtracker.ui.schedule

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
import androidx.compose.material.icons.filled.Add
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary

fun resolveScheduleDisplayNames(
    schedule: ScheduleDto,
    people: List<PersonDto>,
    medications: List<MedicationDto>
): Pair<String, String?> {
    val medication = schedule.medicationId?.let { medId -> medications.firstOrNull { it.id == medId } }
    val person = schedule.personId?.let { personId -> people.firstOrNull { it.id == personId } }
    val medicationName = medication?.displayName
        ?: medication?.name
        ?: schedule.medicationId?.let { "Medication #$it" }
        ?: "Medication"
    return medicationName to person?.name
}

@Composable
fun ScheduleListScreen(
    uiState: ScheduleUiState,
    onTogglePause: (ScheduleDto) -> Unit,
    onAddClick: () -> Unit,
    modifier: Modifier = Modifier,
    people: List<PersonDto> = emptyList(),
    medications: List<MedicationDto> = emptyList()
) {
    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 16.dp, vertical = 12.dp)
        ) {
            uiState.errorMessage?.let { error ->
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

            uiState.successMessage?.let { success ->
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

            if (uiState.isLoading && uiState.schedules.isEmpty()) {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(color = MedTrackerPrimary)
                }
            } else if (uiState.schedules.isEmpty()) {
                Text(
                    text = "No schedules defined yet",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(vertical = 24.dp)
                )
            } else {
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    contentPadding = PaddingValues(bottom = 80.dp)
                ) {
                    items(uiState.schedules, key = { it.id ?: it.portableId ?: "" }) { schedule ->
                        val (medicationName, personName) = resolveScheduleDisplayNames(schedule, people, medications)
                        ScheduleCard(
                            schedule = schedule,
                            medicationName = medicationName,
                            personName = personName,
                            onTogglePause = { onTogglePause(schedule) }
                        )
                    }
                }
            }
        }

        FloatingActionButton(
            onClick = onAddClick,
            containerColor = MedTrackerPrimary,
            contentColor = MaterialTheme.colorScheme.onPrimary,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(16.dp)
        ) {
            Icon(Icons.Default.Add, contentDescription = "Add Schedule")
        }
    }
}

@Composable
fun ScheduleCard(
    schedule: ScheduleDto,
    onTogglePause: () -> Unit,
    medicationName: String? = null,
    personName: String? = null
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp)
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = medicationName ?: "Dose: ${schedule.doseAmount ?: 1.0} ${schedule.doseUnit ?: "tablet"}",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.SemiBold
                    )
                    if (!personName.isNullOrBlank()) {
                        Text(
                            text = "For $personName",
                            style = MaterialTheme.typography.labelMedium,
                            color = MedTrackerPrimary,
                            fontWeight = FontWeight.SemiBold
                        )
                    }
                }

                if (schedule.paused) {
                    Badge(containerColor = Color(0xFFFFF3CD)) {
                        Text("Paused", color = Color(0xFF856404))
                    }
                } else if (schedule.active) {
                    Badge(containerColor = MaterialTheme.colorScheme.primaryContainer) {
                        Text("Active", color = MaterialTheme.colorScheme.onPrimaryContainer)
                    }
                }
            }

            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "Dose: ${schedule.doseAmount ?: 1.0} ${schedule.doseUnit ?: "tablet"} • ${schedule.frequency ?: "daily"}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            if (!schedule.startDate.isNullOrBlank()) {
                Text(
                    text = "Started: ${schedule.startDate}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            OutlinedButton(
                onClick = onTogglePause,
                modifier = Modifier.align(Alignment.End)
            ) {
                Text(if (schedule.paused) "Resume Schedule" else "Pause Schedule")
            }
        }
    }
}
