package io.damacus.medtracker.ui.dashboard

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.automirrored.filled.InsertDriveFile
import androidx.compose.material.icons.automirrored.filled.ShowChart
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.AdminPanelSettings
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.CheckCircleOutline
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Inventory
import androidx.compose.material.icons.filled.Inventory2
import androidx.compose.material.icons.filled.LocationOn
import androidx.compose.material.icons.filled.Medication
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.PersonAdd
import androidx.compose.material.icons.filled.Schedule
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.UnfoldMore
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.AssistChip
import androidx.compose.material3.AssistChipDefaults
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.model.AiSuggestionDto
import io.damacus.medtracker.data.model.DashboardScheduleItem
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary
import io.damacus.medtracker.ui.theme.NoticeAmberBg
import io.damacus.medtracker.ui.theme.NoticeAmberBorder
import io.damacus.medtracker.ui.theme.NoticeAmberText
import io.damacus.medtracker.ui.theme.StatEmeraldGreen
import io.damacus.medtracker.ui.theme.StatRoyalBlue
import io.damacus.medtracker.ui.theme.StatWineRed
import java.time.LocalDate
import java.time.LocalTime
import java.time.format.DateTimeFormatter
import java.util.Locale

@Composable
fun DashboardScreen(
    session: AppSession,
    uiState: DashboardUiState,
    modifier: Modifier = Modifier,
    onSelectPerson: (Long?) -> Unit,
    onRecordDose: (ScheduleDto) -> Unit,
    onAddPersonClick: () -> Unit = {},
    onAddMedicationClick: () -> Unit = {},
    onDismissMessage: () -> Unit = {}
) {
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(uiState.errorMessage) {
        uiState.errorMessage?.let {
            snackbarHostState.showSnackbar(it)
            onDismissMessage()
        }
    }

    LaunchedEffect(uiState.actionSuccessMessage) {
        uiState.actionSuccessMessage?.let {
            snackbarHostState.showSnackbar(it)
            onDismissMessage()
        }
    }

    Box(modifier = modifier.fillMaxSize()) {
        if (uiState.isLoading) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                CircularProgressIndicator(color = MedTrackerPrimary)
            }
        } else {
            val dashboard = uiState.dashboardData
            val scheduleItems = dashboard.displayScheduleItems
            val user = session.user
            val activePerson = dashboard.selectedPerson
            val greetingName = activePerson?.name?.split(" ")?.firstOrNull()
                ?: user?.name?.split(" ")?.firstOrNull()
                ?: "User"

            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 88.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                // 1. Demo Environment Notice Banner
                item {
                    DemoNoticeBanner()
                }

                // 2. Greeting & Active Person Card
                item {
                    GreetingAndPersonHeader(
                        greetingName = greetingName,
                        people = dashboard.people,
                        selectedPerson = activePerson,
                        userName = user?.name ?: "User",
                        onSelectPerson = onSelectPerson,
                        onAddPersonClick = onAddPersonClick
                    )
                }

                // 3. Overview Metric Stat Cards Row
                item {
                    val totalDue = scheduleItems.count { it.isDueNow }
                    val tasksCount = scheduleItems.size
                    val nextDueText = if (scheduleItems.isEmpty()) {
                        "None today"
                    } else {
                        scheduleItems.firstOrNull { it.isDueNow }?.schedule?.doseCycle ?: "Due now"
                    }
                    MetricStatsCardsRow(
                        nextDueText = nextDueText,
                        dueNowCount = totalDue,
                        tasksLeftCount = tasksCount
                    )
                }

                // 4. Today's Schedule Section (Full Width)
                item {
                    TodaysScheduleSection(
                        scheduleItems = scheduleItems,
                        takingScheduleId = uiState.takingScheduleId,
                        onRecordDose = onRecordDose
                    )
                }

                // 5. Inventory & Stock Alerts Section (Full Width)
                item {
                    StockInventorySection(
                        lowStockMeds = dashboard.lowStockMedications,
                        onAddMedicationClick = onAddMedicationClick
                    )
                }

                // 6. Smart Insights Section (Full Width)
                item {
                    SmartInsightsSection(suggestions = dashboard.aiSuggestions)
                }

                // 7. Version Footer
                item {
                    val versionLabel = dashboard.capabilities?.let { "API ${it.apiVersion}" } ?: "App v1.0.0"
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 8.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = versionLabel,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)
                        )
                    }
                }
            }
        }

        // Primary Action FAB - Add Medication
        ExtendedFloatingActionButton(
            onClick = onAddMedicationClick,
            icon = { Icon(imageVector = Icons.Default.Add, contentDescription = null) },
            text = { Text("Add Medication", fontWeight = FontWeight.Bold) },
            containerColor = MedTrackerPrimary,
            contentColor = Color.White,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(16.dp)
        )

        SnackbarHost(
            hostState = snackbarHostState,
            modifier = Modifier.align(Alignment.BottomCenter)
        )
    }
}

@Composable
internal fun SidebarNavigation(
    session: AppSession,
    activeSection: String,
    modifier: Modifier = Modifier,
    onNavigateToProfile: () -> Unit,
    onNavigateToSection: (String) -> Unit = {},
    onLogoutClick: () -> Unit
) {
    val user = session.user
    val userName = user?.name ?: "User"
    val userRole = user?.role?.replaceFirstChar { it.uppercase() } ?: "Owner"
    val userInitials = userName
        .split(" ")
        .mapNotNull { it.firstOrNull()?.toString() }
        .take(2)
        .joinToString("")
        .ifBlank { "U" }

    Surface(
        modifier = modifier.fillMaxHeight(),
        color = MaterialTheme.colorScheme.surface,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.6f))
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.SpaceBetween
        ) {
            Column {
                // MedTracker Brand Logo
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 12.dp)
                ) {
                    Surface(
                        shape = RoundedCornerShape(12.dp),
                        color = MedTrackerPrimary,
                        modifier = Modifier.size(36.dp)
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Text(
                                text = "M",
                                color = Color.White,
                                fontWeight = FontWeight.Black,
                                fontSize = 18.sp
                            )
                        }
                    }
                    Spacer(modifier = Modifier.width(12.dp))
                    Text(
                        text = "MedTracker",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }

                Spacer(modifier = Modifier.height(16.dp))

                // Navigation Items
                val navItems = listOf(
                    NavigationItem("Dashboard", Icons.Default.Schedule, isSelected = activeSection == "Home" || activeSection == "Dashboard"),
                    NavigationItem("Inventory", Icons.Default.Inventory2, isSelected = activeSection == "Meds" || activeSection == "Inventory"),
                    NavigationItem("Locations", Icons.Default.LocationOn, isSelected = activeSection == "Locations"),
                    NavigationItem("People", Icons.Default.Person, isSelected = activeSection == "Family" || activeSection == "People"),
                    NavigationItem("Medication Finder", Icons.Default.Search, isSelected = activeSection == "Finder" || activeSection == "Medication Finder"),
                    NavigationItem("Medicine reviews", Icons.AutoMirrored.Filled.InsertDriveFile, isSelected = activeSection == "Reviews" || activeSection == "Medicine reviews"),
                    NavigationItem("Reports", Icons.Default.Warning, isSelected = activeSection == "Reports"),
                    NavigationItem("Administration", Icons.Default.AdminPanelSettings, isSelected = activeSection == "Admin" || activeSection == "Administration")
                )

                navItems.forEach { item ->
                    val isSelected = item.isSelected
                    Surface(
                        shape = RoundedCornerShape(12.dp),
                        color = if (isSelected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 2.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .clickable { onNavigateToSection(item.label) }
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 14.dp, vertical = 10.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = item.icon,
                                contentDescription = item.label,
                                tint = if (isSelected) MedTrackerPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.size(20.dp)
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Text(
                                text = item.label,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Medium,
                                color = if (isSelected) MedTrackerPrimary else MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }

            // Bottom User Profile & Sign Out
            Column(modifier = Modifier.fillMaxWidth()) {
                Surface(
                    shape = RoundedCornerShape(14.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(14.dp))
                        .clickable { onNavigateToProfile() }
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Surface(
                            shape = CircleShape,
                            color = MaterialTheme.colorScheme.primaryContainer,
                            modifier = Modifier.size(36.dp)
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                Text(
                                    text = userInitials,
                                    style = MaterialTheme.typography.labelMedium,
                                    fontWeight = FontWeight.Bold,
                                    color = MedTrackerPrimary
                                )
                            }
                        }
                        Spacer(modifier = Modifier.width(10.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = userName,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                            Text(
                                text = userRole,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(12.dp))

                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(8.dp))
                        .clickable { onLogoutClick() }
                        .padding(horizontal = 8.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Filled.ExitToApp,
                        contentDescription = "Sign Out",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(18.dp)
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = "Sign Out",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontWeight = FontWeight.Medium
                    )
                }
            }
        }
    }
}

private data class NavigationItem(
    val label: String,
    val icon: ImageVector,
    val isSelected: Boolean
)

<<<<<<< Updated upstream
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DashboardMainContent(
    session: AppSession,
    uiState: DashboardUiState,
    isWideScreen: Boolean,
    snackbarHostState: SnackbarHostState,
    modifier: Modifier = Modifier,
    onOpenDrawer: (() -> Unit)? = null,
    onRefresh: () -> Unit,
    onSelectPerson: (Long?) -> Unit,
    onRecordDose: (ScheduleDto) -> Unit,
    onNavigateToProfile: () -> Unit
) {
    Scaffold(
        modifier = modifier,
        snackbarHost = { SnackbarHost(snackbarHostState) },
        topBar = {
            if (!isWideScreen) {
                TopAppBar(
                    title = {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Surface(
                                shape = RoundedCornerShape(8.dp),
                                color = MedTrackerPrimary,
                                modifier = Modifier.size(28.dp)
                            ) {
                                Box(contentAlignment = Alignment.Center) {
                                    Text(
                                        text = "M",
                                        color = Color.White,
                                        fontWeight = FontWeight.Bold,
                                        fontSize = 14.sp
                                    )
                                }
                            }
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = "MedTracker",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold
                            )
                        }
                    },
                    navigationIcon = {
                        IconButton(onClick = { onOpenDrawer?.invoke() }) {
                            Icon(imageVector = Icons.Default.Menu, contentDescription = "Menu")
                        }
                    },
                    actions = {
                        IconButton(onClick = onRefresh) {
                            Icon(imageVector = Icons.Default.Refresh, contentDescription = "Refresh")
                        }
                        IconButton(onClick = onNavigateToProfile) {
                            Icon(imageVector = Icons.Default.Person, contentDescription = "Profile")
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.background,
                        titleContentColor = MaterialTheme.colorScheme.onBackground
                    )
                )
            }
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { innerPadding ->
        if (uiState.isLoading) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
                contentAlignment = Alignment.Center
            ) {
                CircularProgressIndicator(color = MedTrackerPrimary)
            }
        } else {
            val dashboard = uiState.dashboardData
            val scheduleItems = dashboard.displayScheduleItems
            val user = session.user
            val activePerson = dashboard.selectedPerson
            val greetingName = activePerson?.name?.split(" ")?.firstOrNull()
                ?: user?.name?.split(" ")?.firstOrNull()
                ?: "Alex"

            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
                contentPadding = PaddingValues(horizontal = 24.dp, vertical = 20.dp),
                verticalArrangement = Arrangement.spacedBy(20.dp)
            ) {
                // 1. Demo Environment Notice Banner
                item {
                    DemoNoticeBanner()
                }

                // 2. Greeting & Top Action Buttons
                item {
                    GreetingHeader(
                        greetingName = greetingName,
                        onAddPerson = { /* Action handled */ },
                        onAddMedication = { /* Action handled */ }
                    )
                }

                // 3. Person Switcher Bar
                item {
                    PersonSwitcherBar(
                        people = dashboard.people,
                        selectedPerson = activePerson,
                        userName = user?.name ?: "Alex Demo",
                        onSelectPerson = onSelectPerson
                    )
                }

                // 4. Metric Stat Cards Row
                item {
                    val totalDue = scheduleItems.count { it.isDueNow }
                    val tasksCount = scheduleItems.size
                    MetricStatsCardsRow(
                        nextDueText = if (scheduleItems.isEmpty()) "None today" else "18:00",
                        dueNowCount = totalDue,
                        tasksLeftCount = tasksCount
                    )
                }

                // 5. Two-column / Grid: Today's Schedule + Stock Inventory
                item {
                    ScheduleAndInventorySection(
                        scheduleItems = scheduleItems,
                        takingScheduleId = uiState.takingScheduleId,
                        onRecordDose = onRecordDose,
                        pauseController = uiState.pauseController
                    )
                }

                uiState.pauseController?.let { controller ->
                    item {
                        val pauseState by controller.state.collectAsState()
                        MedicationPauseControls(pauseState, dashboard.selectedPersonId, controller, scheduleItems.mapNotNull { it.schedule.portableId }.toSet())
                    }
                }

                // 6. Smart Insights Card
                item {
                    SmartInsightsCard()
                }

                // 7. Version Footer
                item {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 16.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "v0.5.20",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)
                        )
                    }
                }
            }
        }
    }
}

=======
>>>>>>> Stashed changes
@Composable
private fun DemoNoticeBanner() {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        color = NoticeAmberBg,
        border = BorderStroke(1.dp, NoticeAmberBorder)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.Info,
                contentDescription = null,
                tint = NoticeAmberText,
                modifier = Modifier.size(20.dp)
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column {
                Text(
                    text = "Demo Environment",
                    style = MaterialTheme.typography.labelMedium,
                    fontWeight = FontWeight.Bold,
                    color = NoticeAmberText
                )
                Text(
                    text = "Contains disposable synthetic data. Reset weekly.",
                    style = MaterialTheme.typography.bodySmall,
                    color = NoticeAmberText
                )
            }
        }
    }
}

@Composable
private fun GreetingAndPersonHeader(
    greetingName: String,
    people: List<PersonDto>,
    selectedPerson: PersonDto?,
    userName: String,
    onSelectPerson: (Long?) -> Unit,
    onAddPersonClick: () -> Unit
) {
    val today = LocalDate.now()
    val dateText = today.format(DateTimeFormatter.ofPattern("EEEE, MMM dd", Locale.ENGLISH)).uppercase()
    val hour = LocalTime.now().hour
    val greetingPrefix = when {
        hour < 12 -> "Good morning"
        hour < 17 -> "Good afternoon"
        else -> "Good evening"
    }

    var dropdownExpanded by remember { mutableStateOf(false) }
    val activeDisplayName = selectedPerson?.name ?: userName
    val initials = activeDisplayName
        .split(" ")
        .mapNotNull { it.firstOrNull()?.toString() }
        .take(2)
        .joinToString("")
        .ifBlank { "U" }

    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        Column {
            Text(
                text = dateText,
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.8f),
                letterSpacing = 1.2.sp
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = "$greetingPrefix, $greetingName",
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
        }

        // Person Switcher Card
        ElevatedCard(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.elevatedCardColors(
                containerColor = MaterialTheme.colorScheme.surface
            )
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Surface(
                        shape = CircleShape,
                        color = MaterialTheme.colorScheme.primaryContainer,
                        modifier = Modifier.size(40.dp)
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Text(
                                text = initials,
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = MedTrackerPrimary
                            )
                        }
                    }
                    Spacer(modifier = Modifier.width(12.dp))
                    Column {
                        Text(
                            text = activeDisplayName,
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = if (selectedPerson == null) "Viewing All Family" else "Active Profile",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                Box {
                    AssistChip(
                        onClick = { dropdownExpanded = true },
                        label = { Text("Switch") },
                        trailingIcon = {
                            Icon(
                                imageVector = Icons.Default.UnfoldMore,
                                contentDescription = "Switch person",
                                modifier = Modifier.size(16.dp)
                            )
                        },
                        colors = AssistChipDefaults.assistChipColors(
                            containerColor = MaterialTheme.colorScheme.surfaceVariant
                        )
                    )

                    DropdownMenu(
                        expanded = dropdownExpanded,
                        onDismissRequest = { dropdownExpanded = false }
                    ) {
                        DropdownMenuItem(
                            text = { Text("All Family Members") },
                            onClick = {
                                dropdownExpanded = false
                                onSelectPerson(null)
                            }
                        )
                        people.forEach { person ->
                            DropdownMenuItem(
                                text = { Text(person.name) },
                                onClick = {
                                    dropdownExpanded = false
                                    onSelectPerson(person.id)
                                }
                            )
                        }
                        DropdownMenuItem(
                            text = {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Icon(imageVector = Icons.Default.PersonAdd, contentDescription = null, modifier = Modifier.size(18.dp))
                                    Spacer(modifier = Modifier.width(8.dp))
                                    Text("Add Family Member")
                                }
                            },
                            onClick = {
                                dropdownExpanded = false
                                onAddPersonClick()
                            }
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun MetricStatsCardsRow(
    nextDueText: String,
    dueNowCount: Int,
    tasksLeftCount: Int
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        // NEXT DUE Card
        OutlinedCard(
            modifier = Modifier.weight(1f),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.outlinedCardColors(
                containerColor = MaterialTheme.colorScheme.surface
            )
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 12.dp, horizontal = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "NEXT DUE",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 10.sp
                    )
                    Spacer(modifier = Modifier.width(3.dp))
                    Icon(
                        imageVector = Icons.Default.Schedule,
                        contentDescription = null,
                        tint = StatWineRed,
                        modifier = Modifier.size(12.dp)
                    )
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = nextDueText,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = StatWineRed,
                    maxLines = 1
                )
            }
        }

        // DUE NOW Card
        OutlinedCard(
            modifier = Modifier.weight(1f),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.outlinedCardColors(
                containerColor = MaterialTheme.colorScheme.surface
            )
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 12.dp, horizontal = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "DUE NOW",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 10.sp
                    )
                    Spacer(modifier = Modifier.width(3.dp))
                    Icon(
                        imageVector = Icons.Default.DateRange,
                        contentDescription = null,
                        tint = StatEmeraldGreen,
                        modifier = Modifier.size(12.dp)
                    )
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = dueNowCount.toString(),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = StatEmeraldGreen
                )
            }
        }

        // TODAY'S TASKS Card
        OutlinedCard(
            modifier = Modifier.weight(1f),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.outlinedCardColors(
                containerColor = MaterialTheme.colorScheme.surface
            )
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 12.dp, horizontal = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "TASKS LEFT",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 10.sp
                    )
                    Spacer(modifier = Modifier.width(3.dp))
                    Icon(
                        imageVector = Icons.Default.CheckCircleOutline,
                        contentDescription = null,
                        tint = StatRoyalBlue,
                        modifier = Modifier.size(12.dp)
                    )
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = tasksLeftCount.toString(),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = StatRoyalBlue
                )
            }
        }
    }
}

@Composable
private fun TodaysScheduleSection(
    scheduleItems: List<DashboardScheduleItem>,
    takingScheduleId: Long?,
    onRecordDose: (ScheduleDto) -> Unit,
    pauseController: MedicationPauseController? = null
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Today's Schedule",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
            if (scheduleItems.isNotEmpty()) {
                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = MaterialTheme.colorScheme.primaryContainer
                ) {
<<<<<<< Updated upstream
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "No medications scheduled for today.",
                            style = MaterialTheme.typography.bodyMedium,
                            fontStyle = FontStyle.Italic,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            } else {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    scheduleItems.forEach { item ->
                        ScheduleDoseItemRow(
                            item = item,
                            isTaking = takingScheduleId == item.schedule.id,
                            onTakeClick = { onRecordDose(item.schedule) }
                        )
                        pauseController?.let { SchedulePauseActions(it, item.schedule.portableId) }
                    }
=======
                    Text(
                        text = "${scheduleItems.size} doses",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MedTrackerPrimary,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
>>>>>>> Stashed changes
                }
            }
        }

        Spacer(modifier = Modifier.height(10.dp))

        if (scheduleItems.isEmpty()) {
            OutlinedCard(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                colors = CardDefaults.outlinedCardColors(containerColor = MaterialTheme.colorScheme.surface)
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Icon(
                        imageVector = Icons.Default.CheckCircleOutline,
                        contentDescription = null,
                        tint = StatEmeraldGreen,
                        modifier = Modifier.size(36.dp)
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "All caught up for today!",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    Text(
                        text = "No pending medication doses scheduled.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        } else {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                scheduleItems.forEach { item ->
                    ScheduleDoseCard(
                        item = item,
                        isTaking = takingScheduleId == item.schedule.id,
                        onTakeClick = { onRecordDose(item.schedule) }
                    )
                }
            }
        }
    }
}

@Composable
private fun ScheduleDoseCard(
    item: DashboardScheduleItem,
    isTaking: Boolean,
    onTakeClick: () -> Unit
) {
    val schedule = item.schedule
    val med = item.medication

    ElevatedCard(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Row(
                modifier = Modifier.weight(1f),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = MaterialTheme.colorScheme.primaryContainer,
                    modifier = Modifier.size(44.dp)
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            imageVector = Icons.Default.Medication,
                            contentDescription = null,
                            tint = MedTrackerPrimary,
                            modifier = Modifier.size(24.dp)
                        )
                    }
                }

                Spacer(modifier = Modifier.width(14.dp))

                Column {
                    Text(
                        text = med?.displayName ?: med?.name ?: "Medication",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                    Text(
                        text = "${schedule.doseAmount ?: med?.doseAmount ?: 1.0} ${schedule.doseUnit ?: med?.doseUnit ?: "tablet"} • ${schedule.frequency ?: "daily"}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    item.person?.name?.let { personName ->
                        Text(
                            text = "For $personName",
                            style = MaterialTheme.typography.labelSmall,
                            color = MedTrackerPrimary,
                            fontWeight = FontWeight.SemiBold
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.width(10.dp))

            Button(
                onClick = onTakeClick,
                enabled = !isTaking,
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(containerColor = MedTrackerPrimary)
            ) {
                if (isTaking) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(16.dp),
                        strokeWidth = 2.dp,
                        color = Color.White
                    )
                } else {
                    Icon(
                        imageVector = Icons.Default.Check,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp)
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(text = "Take", fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}

@Composable
private fun StockInventorySection(
	lowStockMeds: List<MedicationDto>,
	onAddMedicationClick: () -> Unit
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = "Inventory & Stock",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface
        )
        Spacer(modifier = Modifier.height(10.dp))

        OutlinedCard(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.outlinedCardColors(containerColor = MaterialTheme.colorScheme.surface)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Inventory,
                            contentDescription = null,
                            tint = MedTrackerPrimary,
                            modifier = Modifier.size(22.dp)
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Column {
                            Text(
                                text = if (lowStockMeds.isEmpty()) "Stock Level Good" else "${lowStockMeds.size} Low Stock Alert(s)",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                            Text(
                                text = if (lowStockMeds.isEmpty()) "All tracked medications have adequate supply." else lowStockMeds.joinToString(", ") { it.name },
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(12.dp))

                OutlinedButton(
                    onClick = onAddMedicationClick,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Text("Manage Inventory & Refills", fontWeight = FontWeight.SemiBold)
                }
            }
        }
    }
}

@Composable
private fun SmartInsightsSection(suggestions: List<AiSuggestionDto> = emptyList()) {
    val topSuggestion = suggestions.firstOrNull()

    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = "Smart Insights",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface
        )
        Spacer(modifier = Modifier.height(10.dp))

        OutlinedCard(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.outlinedCardColors(containerColor = MaterialTheme.colorScheme.surface)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Surface(
                        shape = RoundedCornerShape(10.dp),
                        color = MaterialTheme.colorScheme.primaryContainer,
                        modifier = Modifier.size(36.dp)
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ShowChart,
                                contentDescription = null,
                                tint = MedTrackerPrimary,
                                modifier = Modifier.size(20.dp)
                            )
                        }
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = topSuggestion?.title ?: "Learning your routine",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }

                Text(
                    text = topSuggestion?.text ?: "Keep logging doses regularly so MedTracker can detect routine patterns.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}
