package io.damacus.medtracker

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.animation.Crossfade
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Medication
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.People
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.damacus.medtracker.data.model.HouseholdAdminSettingsDto
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.network.LiveNetworkMonitor
import io.damacus.medtracker.data.network.NetworkMonitor
import io.damacus.medtracker.data.offline.OfflineQueueRepository
import io.damacus.medtracker.ui.MainViewModel
import io.damacus.medtracker.ui.admin.AdministrationScreen
import io.damacus.medtracker.ui.admin.AdministrationViewModel
import io.damacus.medtracker.ui.common.OfflineSyncBanner
import io.damacus.medtracker.ui.dashboard.DashboardScreen
import io.damacus.medtracker.ui.dashboard.DashboardViewModel
import io.damacus.medtracker.ui.dashboard.SidebarNavigation
import io.damacus.medtracker.ui.dose.StockRemovalDialog
import io.damacus.medtracker.ui.healthevents.HealthEventFormDialog
import io.damacus.medtracker.ui.healthevents.HealthEventsScreen
import io.damacus.medtracker.ui.healthevents.HealthEventsViewModel
import io.damacus.medtracker.ui.household.HouseholdMembersScreen
import io.damacus.medtracker.ui.household.HouseholdViewModel
import io.damacus.medtracker.ui.household.InvitationDialog
import io.damacus.medtracker.ui.location.LocationListScreen
import io.damacus.medtracker.ui.lookup.MedicationLookupScreen
import io.damacus.medtracker.ui.lookup.MedicationLookupViewModel
import io.damacus.medtracker.ui.medication.MedicationDetailScreen
import io.damacus.medtracker.ui.medication.MedicationFormScreen
import io.damacus.medtracker.ui.medication.MedicationListScreen
import io.damacus.medtracker.ui.medication.MedicationViewModel
import io.damacus.medtracker.ui.profile.ProfileScreen
import io.damacus.medtracker.ui.reports.ReportsScreen
import io.damacus.medtracker.ui.reports.ReportsViewModel
import io.damacus.medtracker.ui.schedule.ScheduleFormScreen
import io.damacus.medtracker.ui.schedule.ScheduleListScreen
import io.damacus.medtracker.ui.schedule.ScheduleViewModel
import io.damacus.medtracker.ui.theme.MedTrackerPrimary
import io.damacus.medtracker.ui.theme.MedTrackerTheme
import kotlinx.coroutines.launch
import net.openid.appauth.AuthorizationRequest
import net.openid.appauth.AuthorizationResponse
import net.openid.appauth.AuthorizationService
import net.openid.appauth.AuthorizationServiceConfiguration
import net.openid.appauth.CodeVerifierUtil
import net.openid.appauth.ResponseTypeValues

enum class AppDestination(val label: String) {
    Dashboard("Home"),
    Medications("Meds"),
    Schedules("Schedules"),
    Locations("Locations"),
    Household("Family"),
    MedicationFinder("Finder"),
    HealthEvents("Reviews"),
    Reports("Reports"),
    Admin("Admin"),
    Profile("Profile")
}

class MainActivity : ComponentActivity() {

    private lateinit var authorizationService: AuthorizationService
    private val sessionManager by lazy { (application as MedTrackerApplication).sessionManager }
    private val offlineQueueRepository by lazy { OfflineQueueRepository() }
    private val networkMonitor by lazy { LiveNetworkMonitor(applicationContext) }

    private val mainViewModel by viewModels<MainViewModel> {
        MainViewModel.Factory(sessionManager)
    }
    private val dashboardViewModel by viewModels<DashboardViewModel> {
        DashboardViewModel.Factory(sessionManager)
    }
    private val medicationViewModel by viewModels<MedicationViewModel> {
        MedicationViewModel.Factory(sessionManager)
    }
    private val scheduleViewModel by viewModels<ScheduleViewModel> {
        ScheduleViewModel.Factory(sessionManager)
    }
    private val householdViewModel by viewModels<HouseholdViewModel> {
        HouseholdViewModel.Factory(sessionManager)
    }
    private val healthEventsViewModel by viewModels<HealthEventsViewModel> {
        HealthEventsViewModel.Factory(sessionManager)
    }
    private val medicationLookupViewModel by viewModels<MedicationLookupViewModel> {
        MedicationLookupViewModel.Factory(sessionManager)
    }
    private val administrationViewModel by viewModels<AdministrationViewModel> {
        AdministrationViewModel.Factory(sessionManager)
    }
    private val reportsViewModel by viewModels<ReportsViewModel> {
        ReportsViewModel.Factory(sessionManager)
    }

    private val authorizationResult = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val data = result.data ?: Intent()
        val response = AuthorizationResponse.fromIntent(data)
        if (response == null) {
            mainViewModel.reportAuthenticationError("OIDC authorization did not complete")
            return@registerForActivityResult
        }
        authorizationService.performTokenRequest(response.createTokenExchangeRequest()) { tokenResponse, error ->
            val idToken = tokenResponse?.idToken
            val nonce = response.request.nonce
            val verifier = response.request.codeVerifier
            if (idToken == null || nonce == null || verifier == null) {
                mainViewModel.reportAuthenticationError(error?.errorDescription ?: "OIDC token exchange failed")
            } else {
                mainViewModel.exchangeOidc(idToken, nonce, verifier)
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        authorizationService = AuthorizationService(this)

        setContent {
            MedTrackerTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    MedTrackerApp(
                        mainViewModel = mainViewModel,
                        dashboardViewModel = dashboardViewModel,
                        medicationViewModel = medicationViewModel,
                        scheduleViewModel = scheduleViewModel,
                        householdViewModel = householdViewModel,
                        healthEventsViewModel = healthEventsViewModel,
                        medicationLookupViewModel = medicationLookupViewModel,
                        administrationViewModel = administrationViewModel,
                        reportsViewModel = reportsViewModel,
                        networkMonitor = networkMonitor,
                        offlineQueueRepository = offlineQueueRepository,
                        onOidcSignIn = ::startOidcSignIn
                    )
                }
            }
        }
    }

    override fun onDestroy() {
        authorizationService.dispose()
        super.onDestroy()
    }

    private fun startOidcSignIn() {
        val configuration = AuthorizationServiceConfiguration(
            Uri.parse(BuildConfig.OIDC_AUTHORIZATION_ENDPOINT),
            Uri.parse(BuildConfig.OIDC_TOKEN_ENDPOINT)
        )
        val verifier = CodeVerifierUtil.generateRandomCodeVerifier()
        val request = AuthorizationRequest.Builder(
            configuration,
            BuildConfig.OIDC_CLIENT_ID,
            ResponseTypeValues.CODE,
            Uri.parse(BuildConfig.OIDC_REDIRECT_URI)
        )
            .setScope("openid profile email")
            .setCodeVerifier(
                verifier,
                CodeVerifierUtil.deriveCodeVerifierChallenge(verifier),
                "S256"
            )
            .build()

        authorizationResult.launch(authorizationService.getAuthorizationRequestIntent(request))
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MedTrackerApp(
    mainViewModel: MainViewModel,
    dashboardViewModel: DashboardViewModel,
    medicationViewModel: MedicationViewModel,
    scheduleViewModel: ScheduleViewModel,
    householdViewModel: HouseholdViewModel,
    healthEventsViewModel: HealthEventsViewModel,
    medicationLookupViewModel: MedicationLookupViewModel,
    administrationViewModel: AdministrationViewModel,
    reportsViewModel: ReportsViewModel,
    networkMonitor: NetworkMonitor,
    offlineQueueRepository: OfflineQueueRepository,
    onOidcSignIn: () -> Unit
) {
    val session by mainViewModel.sessionState.collectAsStateWithLifecycle()
    val mainUiState by mainViewModel.uiState.collectAsStateWithLifecycle()
    val collectedDashboardUiState by dashboardViewModel.uiState.collectAsStateWithLifecycle()
    val medUiState by medicationViewModel.uiState.collectAsStateWithLifecycle()
    val schedUiState by scheduleViewModel.uiState.collectAsStateWithLifecycle()
    val hhUiState by householdViewModel.uiState.collectAsStateWithLifecycle()
    val healthUiState by healthEventsViewModel.uiState.collectAsStateWithLifecycle()
    val lookupUiState by medicationLookupViewModel.uiState.collectAsStateWithLifecycle()
    val adminUiState by administrationViewModel.uiState.collectAsStateWithLifecycle()

    val isOnline by networkMonitor.isOnline.collectAsStateWithLifecycle(initialValue = true)
    val pendingMutations by offlineQueueRepository.pendingMutations.collectAsStateWithLifecycle()

    val dashboardUiState = collectedDashboardUiState.forSession(session)
    val sessionRevision = session.revision
    var currentDestination by remember(sessionRevision) { mutableStateOf(AppDestination.Dashboard) }

    var showAddMedicationForm by remember { mutableStateOf(false) }
    var selectedMedication by remember { mutableStateOf<MedicationDto?>(null) }
    var showStockRemovalDialog by remember { mutableStateOf(false) }
    var showAddScheduleForm by remember { mutableStateOf(false) }
    var showInviteDialog by remember { mutableStateOf(false) }
    var showHealthEventDialog by remember { mutableStateOf(false) }

    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
    val scope = rememberCoroutineScope()

    val bottomNavDestinations = listOf(
        AppDestination.Dashboard,
        AppDestination.Medications,
        AppDestination.Schedules,
        AppDestination.Household,
        AppDestination.Profile
    )

    Crossfade(
        targetState = session.isLoggedIn,
        label = "AuthCrossfade"
    ) { loggedIn ->
        if (loggedIn && session.isLoggedIn) {
            ModalNavigationDrawer(
                drawerState = drawerState,
                drawerContent = {
                    ModalDrawerSheet(
                        modifier = Modifier.width(280.dp),
                        drawerContainerColor = MaterialTheme.colorScheme.surface
                    ) {
                        SidebarNavigation(
                            session = session,
                            activeSection = currentDestination.label,
                            onNavigateToProfile = {
                                scope.launch { drawerState.close() }
                                currentDestination = AppDestination.Profile
                            },
                            onNavigateToSection = { section ->
                                scope.launch { drawerState.close() }
                                currentDestination = when (section) {
                                    "Inventory" -> AppDestination.Medications
                                    "Locations" -> AppDestination.Locations
                                    "People" -> AppDestination.Household
                                    "Medication Finder" -> AppDestination.MedicationFinder
                                    "Medicine reviews" -> AppDestination.HealthEvents
                                    "Reports" -> AppDestination.Reports
                                    "Administration" -> AppDestination.Admin
                                    else -> AppDestination.Dashboard
                                }
                            },
                            onLogoutClick = {
                                scope.launch { drawerState.close() }
                                mainViewModel.logout(sessionRevision)
                            },
                            modifier = Modifier.fillMaxHeight()
                        )
                    }
                }
            ) {
                Scaffold(
                    topBar = {
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
                                IconButton(onClick = { scope.launch { drawerState.open() } }) {
                                    Icon(imageVector = Icons.Default.Menu, contentDescription = "Menu")
                                }
                            },
                            actions = {
                                IconButton(onClick = { dashboardViewModel.refresh(sessionRevision) }) {
                                    Icon(imageVector = Icons.Default.Refresh, contentDescription = "Refresh")
                                }
                                IconButton(onClick = { currentDestination = AppDestination.Profile }) {
                                    Icon(imageVector = Icons.Default.Person, contentDescription = "Profile")
                                }
                            },
                            colors = TopAppBarDefaults.topAppBarColors(
                                containerColor = MaterialTheme.colorScheme.background,
                                titleContentColor = MaterialTheme.colorScheme.onBackground
                            )
                        )
                    },
                    bottomBar = {
                        NavigationBar {
                            bottomNavDestinations.forEach { destination ->
                                NavigationBarItem(
                                    selected = (currentDestination == destination),
                                    onClick = { currentDestination = destination },
                                    icon = {
                                        Icon(
                                            imageVector = when (destination) {
                                                AppDestination.Dashboard -> Icons.Default.Home
                                                AppDestination.Medications -> Icons.Default.Medication
                                                AppDestination.Schedules -> Icons.Default.CalendarMonth
                                                AppDestination.Household -> Icons.Default.People
                                                AppDestination.Profile -> Icons.Default.Person
                                                else -> Icons.Default.Home
                                            },
                                            contentDescription = destination.label
                                        )
                                    },
                                    label = { Text(destination.label) }
                                )
                            }
                        }
                    }
                ) { padding ->
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(padding)
                    ) {
                        OfflineSyncBanner(isOnline = isOnline, pendingMutationCount = pendingMutations.size)

                        Crossfade(
                            targetState = currentDestination,
                            label = "MainFlowCrossfade"
                        ) { destination ->
                            when (destination) {
                                AppDestination.Dashboard -> {
                                    DashboardScreen(
                                        session = session,
                                        uiState = dashboardUiState,
                                        onSelectPerson = { personId -> dashboardViewModel.selectPerson(personId, sessionRevision) },
                                        onRecordDose = { schedule -> dashboardViewModel.recordDose(schedule, sessionRevision) },
                                        onAddPersonClick = {
                                            currentDestination = AppDestination.Household
                                            showInviteDialog = true
                                        },
                                        onAddMedicationClick = {
                                            currentDestination = AppDestination.Medications
                                            showAddMedicationForm = true
                                        },
                                        onDismissMessage = { dashboardViewModel.clearMessages(sessionRevision) }
                                    )
                                }
                                AppDestination.Medications -> {
                                    if (showAddMedicationForm) {
                                        MedicationFormScreen(
                                            onBackClick = { showAddMedicationForm = false },
                                            onSubmit = { payload, locId ->
                                                medicationViewModel.createMedication(payload, locId) {
                                                    showAddMedicationForm = false
                                                }
                                            }
                                        )
                                    } else if (selectedMedication != null) {
                                        val currentMed = medUiState.medications.find { it.id == selectedMedication?.id } ?: selectedMedication!!
                                        if (showStockRemovalDialog) {
                                            StockRemovalDialog(
                                                medicationName = currentMed.name,
                                                onDismiss = { showStockRemovalDialog = false },
                                                onConfirmRemoval = { qty, reason ->
                                                    val medId = currentMed.id
                                                    if (medId != null) {
                                                        medicationViewModel.recordStockRemoval(medId, qty, reason) {
                                                            showStockRemovalDialog = false
                                                        }
                                                    }
                                                }
                                            )
                                        }
                                        MedicationDetailScreen(
                                            medication = currentMed,
                                            onBackClick = { selectedMedication = null },
                                            onAdjustStockClick = { showStockRemovalDialog = true }
                                        )
                                    } else {
                                        MedicationListScreen(
                                            uiState = medUiState,
                                            onSearchChange = { medicationViewModel.updateSearchQuery(it) },
                                            onMedicationClick = { selectedMedication = it },
                                            onAddClick = { showAddMedicationForm = true }
                                        )
                                    }
                                }
                                AppDestination.Schedules -> {
                                    if (showAddScheduleForm) {
                                        ScheduleFormScreen(
                                            people = dashboardUiState.dashboardData.people,
                                            medications = dashboardUiState.dashboardData.medications,
                                            onBackClick = { showAddScheduleForm = false },
                                            onSubmit = { payload ->
                                                scheduleViewModel.createSchedule(payload) {
                                                    showAddScheduleForm = false
                                                }
                                            }
                                        )
                                    } else {
                                        ScheduleListScreen(
                                            uiState = schedUiState,
                                            onTogglePause = { scheduleViewModel.togglePauseSchedule(it) },
                                            onAddClick = { showAddScheduleForm = true }
                                        )
                                    }
                                }
                                AppDestination.Locations -> {
                                    LocationListScreen(
                                        locations = hhUiState.locations,
                                        isLoading = hhUiState.isLoading,
                                        onAddClick = {}
                                    )
                                }
                                AppDestination.Household -> {
                                    if (showInviteDialog) {
                                        InvitationDialog(
                                            onDismiss = { showInviteDialog = false },
                                            onConfirmSend = { email, role ->
                                                householdViewModel.createInvitation(email, role) {
                                                    showInviteDialog = false
                                                }
                                            }
                                        )
                                    }
                                    HouseholdMembersScreen(
                                        people = dashboardUiState.dashboardData.people,
                                        invitations = hhUiState.invitations,
                                        isLoading = hhUiState.isLoading,
                                        onInviteClick = { showInviteDialog = true }
                                    )
                                }
                                AppDestination.MedicationFinder -> {
                                    MedicationLookupScreen(
                                        results = lookupUiState.results,
                                        isLoading = lookupUiState.isLoading,
                                        onSearch = { query -> medicationLookupViewModel.search(query) }
                                    )
                                }
                                AppDestination.HealthEvents -> {
                                    if (showHealthEventDialog) {
                                        HealthEventFormDialog(
                                            people = dashboardUiState.dashboardData.people,
                                            onDismiss = { showHealthEventDialog = false },
                                            onSubmit = { personId, title, notes ->
                                                healthEventsViewModel.createHealthEvent(personId, title, notes) {
                                                    showHealthEventDialog = false
                                                }
                                            }
                                        )
                                    }
                                    HealthEventsScreen(
                                        events = healthUiState.events,
                                        onAddEventClick = { showHealthEventDialog = true }
                                    )
                                }
                                AppDestination.Reports -> {
                                    ReportsScreen(
                                        onExportZipClick = { reportsViewModel.exportBackupZip() }
                                    )
                                }
                                AppDestination.Admin -> {
                                    AdministrationScreen(
                                        adminSettings = adminUiState.adminSettings
                                    )
                                }
                                AppDestination.Profile -> {
                                    ProfileScreen(
                                        session = session,
                                        activePerson = dashboardUiState.dashboardData.selectedPerson,
                                        onBackClick = { currentDestination = AppDestination.Dashboard },
                                        onLogoutClick = { mainViewModel.logout(sessionRevision) }
                                    )
                                }
                            }
                        }
                    }
                }
            }
        } else {
            AuthRoute(
                isLoading = mainUiState.isLoading,
                errorMessage = mainUiState.errorMessage,
                householdSelection = mainUiState.householdSelection,
                viewModel = mainViewModel,
                onOidcSignIn = onOidcSignIn
            )
        }
    }
}
