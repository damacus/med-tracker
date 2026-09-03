package io.damacus.medtracker

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.result.contract.ActivityResultContracts
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
import io.damacus.medtracker.ui.profile.ProfileScreen
import io.damacus.medtracker.ui.theme.MedTrackerTheme
import net.openid.appauth.AuthorizationRequest
import net.openid.appauth.AuthorizationResponse
import net.openid.appauth.AuthorizationService
import net.openid.appauth.AuthorizationServiceConfiguration
import net.openid.appauth.CodeVerifierUtil
import net.openid.appauth.ResponseTypeValues

enum class AppDestination {
    Dashboard,
    Profile
}

class MainActivity : ComponentActivity() {

    private lateinit var authorizationService: AuthorizationService
    private val sessionManager by lazy { SessionManager(applicationContext) }
    private val mainViewModel by viewModels<MainViewModel> {
        MainViewModel.Factory(sessionManager)
    }
    private val dashboardViewModel by viewModels<DashboardViewModel> {
        DashboardViewModel.Factory(sessionManager)
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

@Composable
fun MedTrackerApp(
    mainViewModel: MainViewModel,
    dashboardViewModel: DashboardViewModel,
    onOidcSignIn: () -> Unit
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
            AuthRoute(
                isLoading = mainUiState.isLoading,
                errorMessage = mainUiState.errorMessage,
                viewModel = mainViewModel,
                onOidcSignIn = onOidcSignIn
            )
        }
    }
}
