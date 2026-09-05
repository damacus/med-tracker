package io.damacus.medtracker.ui.dashboard

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
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
import androidx.compose.material.icons.filled.Inventory2
import androidx.compose.material.icons.filled.LocationOn
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Schedule
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.UnfoldMore
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.model.DashboardData
import io.damacus.medtracker.data.model.DashboardScheduleItem
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import io.damacus.medtracker.ui.theme.MedTrackerPrimary
import io.damacus.medtracker.ui.theme.MedTrackerTheme
import io.damacus.medtracker.ui.theme.NoticeAmberBg
import io.damacus.medtracker.ui.theme.NoticeAmberBorder
import io.damacus.medtracker.ui.theme.NoticeAmberText
import io.damacus.medtracker.ui.theme.StatEmeraldGreen
import io.damacus.medtracker.ui.theme.StatRoyalBlue
import io.damacus.medtracker.ui.theme.StatWineRed
import kotlinx.coroutines.launch
import java.time.LocalDate
import java.time.LocalTime
import java.time.format.DateTimeFormatter
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    session: AppSession,
    uiState: DashboardUiState,
    modifier: Modifier = Modifier,
    onRefresh: () -> Unit,
    onSelectPerson: (Long?) -> Unit,
    onRecordDose: (ScheduleDto) -> Unit,
    onNavigateToProfile: () -> Unit,
    onLogoutClick: () -> Unit,
    onDismissMessage: () -> Unit
) {
    val snackbarHostState = remember { SnackbarHostState() }
    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
    val scope = rememberCoroutineScope()

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

    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val isWideScreen = maxWidth >= 840.dp

        if (isWideScreen) {
            // Tablet / Expanded Desktop Layout with Sidebar Navigation
            Row(modifier = Modifier.fillMaxSize()) {
                SidebarNavigation(
                    session = session,
                    activeSection = "Dashboard",
                    onNavigateToProfile = onNavigateToProfile,
                    onLogoutClick = onLogoutClick,
                    modifier = Modifier.width(260.dp)
                )

                DashboardMainContent(
                    session = session,
                    uiState = uiState,
                    isWideScreen = true,
                    snackbarHostState = snackbarHostState,
                    onOpenDrawer = null,
                    onRefresh = onRefresh,
                    onSelectPerson = onSelectPerson,
                    onRecordDose = onRecordDose,
                    onNavigateToProfile = onNavigateToProfile,
                    modifier = Modifier.weight(1f)
                )
            }
        } else {
            // Mobile Compact Layout with Drawer
            ModalNavigationDrawer(
                drawerState = drawerState,
                drawerContent = {
                    ModalDrawerSheet(
                        modifier = Modifier.width(280.dp),
                        drawerContainerColor = MaterialTheme.colorScheme.surface
                    ) {
                        SidebarNavigation(
                            session = session,
                            activeSection = "Dashboard",
                            onNavigateToProfile = {
                                scope.launch { drawerState.close() }
                                onNavigateToProfile()
                            },
                            onLogoutClick = {
                                scope.launch { drawerState.close() }
                                onLogoutClick()
                            },
                            modifier = Modifier.fillMaxHeight()
                        )
                    }
                }
            ) {
                DashboardMainContent(
                    session = session,
                    uiState = uiState,
                    isWideScreen = false,
                    snackbarHostState = snackbarHostState,
                    onOpenDrawer = { scope.launch { drawerState.open() } },
                    onRefresh = onRefresh,
                    onSelectPerson = onSelectPerson,
                    onRecordDose = onRecordDose,
                    onNavigateToProfile = onNavigateToProfile,
                    modifier = Modifier.fillMaxSize()
                )
            }
        }
    }
}

@Composable
private fun SidebarNavigation(
    session: AppSession,
    activeSection: String,
    modifier: Modifier = Modifier,
    onNavigateToProfile: () -> Unit,
    onLogoutClick: () -> Unit
) {
    val user = session.user
    val userName = user?.name ?: "Alex Demo"
    val userRole = user?.role?.replaceFirstChar { it.uppercase() } ?: "Owner"
    val userInitials = userName
        .split(" ")
        .mapNotNull { it.firstOrNull()?.toString() }
        .take(2)
        .joinToString("")
        .ifBlank { "AD" }

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

                // Search Bar with Ctrl K hint
                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 12.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                imageVector = Icons.Default.Search,
                                contentDescription = "Search",
                                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = "Search",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        Surface(
                            shape = RoundedCornerShape(6.dp),
                            color = MaterialTheme.colorScheme.surface,
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
                        ) {
                            Text(
                                text = "Ctrl K",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(20.dp))

                // Navigation Items
                val navItems = listOf(
                    NavigationItem("Dashboard", Icons.Default.Schedule, isSelected = activeSection == "Dashboard"),
                    NavigationItem("Inventory", Icons.Default.Inventory2, isSelected = activeSection == "Inventory"),
                    NavigationItem("Locations", Icons.Default.LocationOn, isSelected = activeSection == "Locations"),
                    NavigationItem("People", Icons.Default.Person, isSelected = activeSection == "People"),
                    NavigationItem("Medication Finder", Icons.Default.Search, isSelected = activeSection == "Medication Finder"),
                    NavigationItem("Medicine reviews", Icons.AutoMirrored.Filled.InsertDriveFile, isSelected = activeSection == "Medicine reviews"),
                    NavigationItem("Reports", Icons.Default.Warning, isSelected = activeSection == "Reports"),
                    NavigationItem("Administration", Icons.Default.AdminPanelSettings, isSelected = activeSection == "Administration")
                )

                navItems.forEach { item ->
                    val isSelected = item.isSelected
                    Surface(
                        shape = RoundedCornerShape(12.dp),
                        color = if (isSelected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 2.dp)
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
                        onRecordDose = onRecordDose
                    )
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
                .padding(horizontal = 16.dp, vertical = 12.dp),
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
                    text = "Demo environment",
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Bold,
                    color = NoticeAmberText
                )
                Text(
                    text = "This environment contains synthetic and disposable data. Every Sunday at 04:15 Europe/London.",
                    style = MaterialTheme.typography.bodySmall,
                    color = NoticeAmberText
                )
            }
        }
    }
}

@Composable
private fun GreetingHeader(
    greetingName: String,
    onAddPerson: () -> Unit,
    onAddMedication: () -> Unit
) {
    val today = LocalDate.now()
    val dateText = today.format(DateTimeFormatter.ofPattern("EEEE, MMM dd", Locale.ENGLISH)).uppercase()
    val hour = LocalTime.now().hour
    val greetingPrefix = when {
        hour < 12 -> "Good morning"
        hour < 17 -> "Good afternoon"
        else -> "Good evening"
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column {
            Text(
                text = dateText,
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.8f),
                letterSpacing = 1.2.sp
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "$greetingPrefix, $greetingName",
                style = MaterialTheme.typography.headlineLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
        }

        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            OutlinedButton(
                onClick = onAddPerson,
                shape = RoundedCornerShape(20.dp),
                border = BorderStroke(1.5.dp, MedTrackerPrimary),
                colors = ButtonDefaults.outlinedButtonColors(
                    contentColor = MedTrackerPrimary
                ),
                contentPadding = PaddingValues(horizontal = 18.dp, vertical = 10.dp)
            ) {
                Text(
                    text = "Add Person",
                    fontWeight = FontWeight.SemiBold
                )
            }

            Button(
                onClick = onAddMedication,
                shape = RoundedCornerShape(20.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = MedTrackerPrimary,
                    contentColor = Color.White
                ),
                contentPadding = PaddingValues(horizontal = 18.dp, vertical = 10.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Add,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "Add Medication",
                    fontWeight = FontWeight.SemiBold
                )
            }
        }
    }
}

@Composable
private fun PersonSwitcherBar(
    people: List<PersonDto>,
    selectedPerson: PersonDto?,
    userName: String,
    onSelectPerson: (Long?) -> Unit
) {
    var dropdownExpanded by remember { mutableStateOf(false) }

    val activeDisplayName = selectedPerson?.name ?: userName
    val initials = activeDisplayName
        .split(" ")
        .mapNotNull { it.firstOrNull()?.toString() }
        .take(2)
        .joinToString("")
        .ifBlank { "AD" }

    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(14.dp),
        color = MaterialTheme.colorScheme.surface,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
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
                    modifier = Modifier.size(38.dp)
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(
                            text = initials,
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.Bold,
                            color = MedTrackerPrimary
                        )
                    }
                }
                Spacer(modifier = Modifier.width(12.dp))
                Text(
                    text = activeDisplayName,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }

            Box {
                Row(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .clickable { dropdownExpanded = true }
                        .padding(horizontal = 8.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "Change person",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Icon(
                        imageVector = Icons.Default.UnfoldMore,
                        contentDescription = "Change person",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(18.dp)
                    )
                }

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
        horizontalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        // NEXT DUE Card (Burgundy)
        Card(
            modifier = Modifier.weight(1f),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surface
            ),
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.Center
                ) {
                    Text(
                        text = "NEXT DUE",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        letterSpacing = 1.sp
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Icon(
                        imageVector = Icons.Default.Schedule,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(15.dp)
                    )
                }
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = nextDueText,
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Black,
                    color = StatWineRed,
                    maxLines = 1
                )
            }
        }

        // DUE NOW Card (Emerald Green)
        Card(
            modifier = Modifier.weight(1f),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surface
            ),
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.Center
                ) {
                    Text(
                        text = "DUE NOW",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        letterSpacing = 1.sp
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Icon(
                        imageVector = Icons.Default.DateRange,
                        contentDescription = null,
                        tint = StatEmeraldGreen,
                        modifier = Modifier.size(15.dp)
                    )
                }
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = dueNowCount.toString(),
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Black,
                    color = StatEmeraldGreen
                )
            }
        }

        // TASKS LEFT Card (Royal Blue)
        Card(
            modifier = Modifier.weight(1f),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.surface
            ),
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.Center
                ) {
                    Text(
                        text = "TASKS LEFT",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        letterSpacing = 1.sp
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Icon(
                        imageVector = Icons.Default.CheckCircleOutline,
                        contentDescription = null,
                        tint = StatRoyalBlue,
                        modifier = Modifier.size(15.dp)
                    )
                }
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = tasksLeftCount.toString(),
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Black,
                    color = StatRoyalBlue
                )
            }
        }
    }
}

@Composable
private fun ScheduleAndInventorySection(
    scheduleItems: List<DashboardScheduleItem>,
    takingScheduleId: Long?,
    onRecordDose: (ScheduleDto) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        // Today's Schedule Card (Left Column)
        Column(modifier = Modifier.weight(1.8f)) {
            Text(
                text = "Today's Schedule",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
            Spacer(modifier = Modifier.height(12.dp))

            if (scheduleItems.isEmpty()) {
                Surface(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(130.dp),
                    shape = RoundedCornerShape(20.dp),
                    color = MaterialTheme.colorScheme.surface.copy(alpha = 0.5f),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                ) {
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
                    }
                }
            }
        }

        // Stock Inventory Card (Right Column)
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = "Stock Inventory",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface
            )
            Spacer(modifier = Modifier.height(12.dp))

            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(130.dp),
                shape = RoundedCornerShape(20.dp),
                color = MaterialTheme.colorScheme.surface,
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(16.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Button(
                        onClick = { /* Navigate or trigger refills */ },
                        shape = RoundedCornerShape(24.dp),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.primaryContainer,
                            contentColor = MedTrackerPrimary
                        ),
                        modifier = Modifier
                            .fillMaxWidth(0.85f)
                            .height(44.dp)
                    ) {
                        Text(
                            text = "ORDER REFILLS",
                            fontWeight = FontWeight.Bold,
                            fontSize = 13.sp,
                            letterSpacing = 0.5.sp
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ScheduleDoseItemRow(
    item: DashboardScheduleItem,
    isTaking: Boolean,
    onTakeClick: () -> Unit
) {
    val schedule = item.schedule
    val med = item.medication

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = med?.displayName ?: med?.name ?: "Medication",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Text(
                    text = "Dose: ${schedule.doseAmount ?: med?.doseAmount ?: ""} ${schedule.doseUnit ?: med?.doseUnit ?: ""}".trim(),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

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
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(text = "Take")
                }
            }
        }
    }
}

@Composable
private fun SmartInsightsCard() {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = "Smart Insights",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface
        )
        Spacer(modifier = Modifier.height(12.dp))

        Surface(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(24.dp),
            color = MaterialTheme.colorScheme.surface,
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(14.dp)
            ) {
                // Waveform pulse icon in rounded box
                Surface(
                    shape = RoundedCornerShape(14.dp),
                    color = MaterialTheme.colorScheme.primaryContainer,
                    modifier = Modifier.size(46.dp)
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ShowChart,
                            contentDescription = "Smart Insights",
                            tint = MedTrackerPrimary,
                            modifier = Modifier.size(24.dp)
                        )
                    }
                }

                Text(
                    text = "Learning your routine",
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface
                )

                Text(
                    text = "Keep logging doses so MedTracker can spot real patterns.",
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                    color = MaterialTheme.colorScheme.onSurface
                )

                Text(
                    text = "Smart Insights waits for enough evidence before calling anything a pattern.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Spacer(modifier = Modifier.height(4.dp))

                Button(
                    onClick = { /* View full smart report */ },
                    shape = RoundedCornerShape(24.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MedTrackerPrimary,
                        contentColor = Color.White
                    ),
                    contentPadding = PaddingValues(horizontal = 24.dp, vertical = 12.dp)
                ) {
                    Text(
                        text = "VIEW FULL REPORT",
                        fontWeight = FontWeight.Bold,
                        fontSize = 13.sp,
                        letterSpacing = 0.5.sp
                    )
                }
            }
        }
    }
}

@Preview(showBackground = true, widthDp = 1200, heightDp = 800)
@Composable
private fun DashboardScreenWidePreview() {
    MedTrackerTheme {
        DashboardScreen(
            session = AppSession(
                serverUrl = "https://med-tracker-canary.damacus.io/",
                sessionPayload = SessionPayload(
                    accessToken = "sample-token",
                    refreshToken = "sample-refresh",
                    me = UserDto(
                        id = 1,
                        emailAddress = "alex.demo@example.com",
                        name = "Alex Demo",
                        role = "owner"
                    ),
                    household = HouseholdDto(
                        id = 42,
                        name = "Demo Household"
                    )
                )
            ),
            uiState = DashboardUiState(
                dashboardData = DashboardData(
                    people = listOf(
                        PersonDto(id = 1, name = "Alex Demo")
                    ),
                    medications = emptyList(),
                    schedules = emptyList(),
                    recentTakes = emptyList()
                )
            ),
            onRefresh = {},
            onSelectPerson = {},
            onRecordDose = {},
            onNavigateToProfile = {},
            onLogoutClick = {},
            onDismissMessage = {}
        )
    }
}

@Preview(showBackground = true, widthDp = 400, heightDp = 800)
@Composable
private fun DashboardScreenMobilePreview() {
    MedTrackerTheme {
        DashboardScreen(
            session = AppSession(
                serverUrl = "https://med-tracker-canary.damacus.io/",
                sessionPayload = SessionPayload(
                    accessToken = "sample-token",
                    refreshToken = "sample-refresh",
                    me = UserDto(
                        id = 1,
                        emailAddress = "alex.demo@example.com",
                        name = "Alex Demo",
                        role = "owner"
                    ),
                    household = HouseholdDto(
                        id = 42,
                        name = "Demo Household"
                    )
                )
            ),
            uiState = DashboardUiState(
                dashboardData = DashboardData(
                    people = listOf(
                        PersonDto(id = 1, name = "Alex Demo")
                    ),
                    medications = emptyList(),
                    schedules = emptyList(),
                    recentTakes = emptyList()
                )
            ),
            onRefresh = {},
            onSelectPerson = {},
            onRecordDose = {},
            onNavigateToProfile = {},
            onLogoutClick = {},
            onDismissMessage = {}
        )
    }
}
