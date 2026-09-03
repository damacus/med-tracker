package io.damacus.medtracker

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.animation.Crossfade
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.ui.MainViewModel
import io.damacus.medtracker.ui.dashboard.DashboardScreen
import io.damacus.medtracker.ui.dashboard.DashboardViewModel
import io.damacus.medtracker.ui.login.LoginScreen
import io.damacus.medtracker.ui.profile.ProfileScreen
import io.damacus.medtracker.ui.theme.MedTrackerTheme

enum class AppDestination {
    Dashboard,
    Profile
}

class MainActivity : ComponentActivity() {

    private val sessionManager by lazy { SessionManager(applicationContext) }
    private val mainViewModel by viewModels<MainViewModel> {
        MainViewModel.Factory(sessionManager)
    }
    private val dashboardViewModel by viewModels<DashboardViewModel> {
        DashboardViewModel.Factory(sessionManager)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            MedTrackerTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    MedTrackerApp(
                        mainViewModel = mainViewModel,
                        dashboardViewModel = dashboardViewModel
                    )
                }
            }
        }
    }
}

@Composable
fun MedTrackerApp(
    mainViewModel: MainViewModel,
    dashboardViewModel: DashboardViewModel
) {
    val session by mainViewModel.sessionState.collectAsStateWithLifecycle()
    val mainUiState by mainViewModel.uiState.collectAsStateWithLifecycle()
    val dashboardUiState by dashboardViewModel.uiState.collectAsStateWithLifecycle()
    var currentDestination by remember { mutableStateOf(AppDestination.Dashboard) }

    Crossfade(
        targetState = session.isLoggedIn,
        label = "AuthCrossfade"
    ) { loggedIn ->
        if (loggedIn) {
            Crossfade(
                targetState = currentDestination,
                label = "MainFlowCrossfade"
            ) { destination ->
                when (destination) {
                    AppDestination.Dashboard -> {
                        DashboardScreen(
                            session = session,
                            uiState = dashboardUiState,
                            onRefresh = { dashboardViewModel.refresh() },
                            onSelectPerson = { personId -> dashboardViewModel.selectPerson(personId) },
                            onRecordDose = { schedule -> dashboardViewModel.recordDose(schedule) },
                            onNavigateToProfile = { currentDestination = AppDestination.Profile },
                            onLogoutClick = { mainViewModel.logout() },
                            onDismissMessage = { dashboardViewModel.clearMessages() }
                        )
                    }
                    AppDestination.Profile -> {
                        ProfileScreen(
                            session = session,
                            activePerson = dashboardUiState.dashboardData.selectedPerson,
                            onBackClick = { currentDestination = AppDestination.Dashboard },
                            onLogoutClick = { mainViewModel.logout() }
                        )
                    }
                }
            }
        } else {
            LoginScreen(
                serverUrl = session.serverUrl,
                isLoading = mainUiState.isLoading,
                errorMessage = mainUiState.errorMessage,
                onServerUrlChanged = { newUrl -> mainViewModel.updateServerUrl(newUrl) },
                onLoginClick = { email, password, serverUrl ->
                    mainViewModel.login(email, password, serverUrl)
                }
            )
        }
    }
}
