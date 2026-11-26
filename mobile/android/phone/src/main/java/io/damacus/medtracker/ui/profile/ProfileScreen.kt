package io.damacus.medtracker.ui.profile

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.filled.AccountCircle
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.Security
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.api.NotificationPreferencesDto
import io.damacus.medtracker.data.model.PersonDto

@Composable
fun ProfileScreen(
    session: AppSession,
    modifier: Modifier = Modifier,
    activePerson: PersonDto? = null,
    notificationPreferences: NotificationPreferencesDto? = null,
    isLoadingPreferences: Boolean = false,
    isSavingPreferences: Boolean = false,
    preferencesError: String? = null,
    preferencesStatusMessage: String? = null,
    onUpdateNotificationPreferences: (NotificationPreferencesDto) -> Unit = {},
    onRetryLoadPreferences: () -> Unit = {},
    onLogoutClick: () -> Unit
) {
    val user = session.user
    val household = session.household
    val context = LocalContext.current
    val requiresNotificationPermission = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU
    var hasNotificationPermission by remember(context) {
        mutableStateOf(
            !requiresNotificationPermission || ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.POST_NOTIFICATIONS
            ) == PackageManager.PERMISSION_GRANTED
        )
    }
    var notificationPermissionDenied by remember { mutableStateOf(false) }
    val notificationPermissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { granted ->
        hasNotificationPermission = granted
        notificationPermissionDenied = !granted
    }

    val displayName = activePerson?.name ?: user?.name ?: "User"
    val email = user?.emailAddress ?: "No email set"
    val role = user?.role?.replaceFirstChar { it.uppercase() } ?: "Member"
    val initials = displayName
        .split(" ")
        .mapNotNull { it.firstOrNull()?.toString() }
        .take(2)
        .joinToString("")
        .ifBlank { "U" }

    Box(modifier = modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Profile Hero Card
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(20.dp),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surface
                ),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(20.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Surface(
                        modifier = Modifier.size(60.dp),
                        shape = CircleShape,
                        color = MaterialTheme.colorScheme.primaryContainer
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Text(
                                text = initials,
                                style = MaterialTheme.typography.headlineSmall,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }
                    }

                    Spacer(modifier = Modifier.width(16.dp))

                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = displayName,
                            style = MaterialTheme.typography.titleLarge,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Spacer(modifier = Modifier.height(2.dp))
                        Text(
                            text = email,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(modifier = Modifier.height(6.dp))
                        Surface(
                            shape = RoundedCornerShape(8.dp),
                            color = MaterialTheme.colorScheme.primary.copy(alpha = 0.1f)
                        ) {
                            Text(
                                text = "$role • ${household?.name ?: "Household"}",
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = FontWeight.SemiBold,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                            )
                        }
                    }
                }
            }

            // Personal Information
            SectionCard(title = "Personal Information") {
                ProfileDetailRow(
                    icon = Icons.Default.AccountCircle,
                    label = "Display Name",
                    value = displayName
                )
                ProfileDetailRow(
                    icon = Icons.Default.Email,
                    label = "Email Address",
                    value = email
                )
                if (activePerson?.personType != null) {
                    ProfileDetailRow(
                        icon = Icons.Default.Info,
                        label = "Person Type",
                        value = activePerson.personType.replaceFirstChar { it.uppercase() }
                    )
                }
                if (activePerson?.age != null) {
                    ProfileDetailRow(
                        icon = Icons.Default.Info,
                        label = "Age",
                        value = "${activePerson.age} years old"
                    )
                }
                ProfileDetailRow(
                    icon = Icons.Default.Home,
                    label = "Active Household",
                    value = household?.name ?: "Default Household"
                )
            }

            // Notification Preferences
            SectionCard(title = "Notification Preferences") {
                when {
                    isLoadingPreferences && notificationPreferences == null -> {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp)
                        ) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                strokeWidth = 2.dp
                            )
                            Text(
                                text = "Loading notification preferences...",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                    notificationPreferences != null -> {
                        val prefs = notificationPreferences
                        PreferenceToggleRow(
                            title = "Medication Reminders",
                            subtitle = "Enable local and push reminders for due doses",
                            checked = prefs.enabled,
                            enabled = !isSavingPreferences,
                            onCheckedChange = { checked ->
                                if (checked && requiresNotificationPermission && !hasNotificationPermission) {
                                    notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
                                }
                                onUpdateNotificationPreferences(prefs.copy(enabled = checked))
                            }
                        )
                        if (requiresNotificationPermission && !hasNotificationPermission && (prefs.enabled || notificationPermissionDenied)) {
                            Surface(
                                shape = RoundedCornerShape(12.dp),
                                color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.5f),
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Column(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(12.dp),
                                    verticalArrangement = Arrangement.spacedBy(8.dp)
                                ) {
                                    Text(
                                        text = "Notifications are blocked on this device. Allow notifications so medication reminders can appear.",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onErrorContainer
                                    )
                                    OutlinedButton(
                                        onClick = {
                                            val settingsIntent = Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).apply {
                                                putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
                                            }
                                            context.startActivity(settingsIntent)
                                        },
                                        shape = RoundedCornerShape(10.dp)
                                    ) {
                                        Text("Open Notification Settings")
                                    }
                                }
                            }
                        }
                        PreferenceToggleRow(
                            title = "Dose Due Alerts",
                            subtitle = "Notify when scheduled doses are due",
                            checked = prefs.doseDueEnabled,
                            enabled = prefs.enabled && !isSavingPreferences,
                            onCheckedChange = { checked ->
                                onUpdateNotificationPreferences(prefs.copy(doseDueEnabled = checked))
                            }
                        )
                        PreferenceToggleRow(
                            title = "Missed Dose Alerts",
                            subtitle = "Alert when a scheduled dose is overdue",
                            checked = prefs.missedDoseEnabled,
                            enabled = prefs.enabled && !isSavingPreferences,
                            onCheckedChange = { checked ->
                                onUpdateNotificationPreferences(prefs.copy(missedDoseEnabled = checked))
                            }
                        )
                        PreferenceToggleRow(
                            title = "Low Stock Alerts",
                            subtitle = "Notify when medication supply runs low",
                            checked = prefs.lowStockEnabled,
                            enabled = prefs.enabled && !isSavingPreferences,
                            onCheckedChange = { checked ->
                                onUpdateNotificationPreferences(prefs.copy(lowStockEnabled = checked))
                            }
                        )
                        PreferenceToggleRow(
                            title = "Private Notification Text",
                            subtitle = "Hide medication names on the lock screen",
                            checked = prefs.privateTextEnabled,
                            enabled = prefs.enabled && !isSavingPreferences,
                            onCheckedChange = { checked ->
                                onUpdateNotificationPreferences(prefs.copy(privateTextEnabled = checked))
                            }
                        )
                    }
                    else -> {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                modifier = Modifier.weight(1f)
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Notifications,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.size(20.dp)
                                )
                                Text(
                                    text = "Notification preferences not loaded.",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            TextButton(onClick = onRetryLoadPreferences) {
                                Text("Reload")
                            }
                        }
                    }
                }

                if (preferencesError != null) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text(
                            text = preferencesError,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.weight(1f)
                        )
                        TextButton(onClick = onRetryLoadPreferences) {
                            Text("Retry")
                        }
                    }
                } else if (preferencesStatusMessage != null) {
                    Text(
                        text = preferencesStatusMessage,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
            }

            // Server & Network Status
            SectionCard(title = "Server Connection") {
                ProfileDetailRow(
                    icon = Icons.Default.Public,
                    label = "Instance URL",
                    value = session.serverUrl
                )
                ProfileDetailRow(
                    icon = Icons.Default.CheckCircle,
                    label = "Session Status",
                    value = if (session.isLoggedIn) "Authenticated" else "Guest"
                )
            }

            // Account Security
            SectionCard(title = "Security & Access") {
                ProfileDetailRow(
                    icon = Icons.Default.Security,
                    label = "Permission Level",
                    value = role
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Sign Out Action Button
            OutlinedButton(
                onClick = onLogoutClick,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp),
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.outlinedButtonColors(
                    contentColor = MaterialTheme.colorScheme.error
                ),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.error.copy(alpha = 0.5f))
            ) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.ExitToApp,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "Sign Out of MedTracker",
                    fontWeight = FontWeight.SemiBold
                )
            }

            Spacer(modifier = Modifier.height(16.dp))
        }
    }
}

@Composable
private fun PreferenceToggleRow(
    title: String,
    subtitle: String,
    checked: Boolean,
    enabled: Boolean,
    onCheckedChange: (Boolean) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Column(modifier = Modifier.weight(1f).padding(end = 12.dp)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        Switch(
            checked = checked,
            enabled = enabled,
            onCheckedChange = onCheckedChange
        )
    }
}

@Composable
private fun SectionCard(
    title: String,
    content: @Composable () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
            content()
        }
    }
}

@Composable
private fun ProfileDetailRow(
    icon: ImageVector,
    label: String,
    value: String
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.size(20.dp)
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column {
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                text = value,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface
            )
        }
    }
}
