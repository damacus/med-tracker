package io.damacus.medtracker

import androidx.compose.runtime.Composable
import io.damacus.medtracker.auth.OidcAuthRoute
import io.damacus.medtracker.ui.MainViewModel

@Composable
fun AuthRoute(
    isLoading: Boolean,
    errorMessage: String?,
    viewModel: MainViewModel,
    onOidcSignIn: () -> Unit
) {
    OidcAuthRoute(
        title = "Sign in securely with MedTracker",
        isLoading = isLoading,
        errorMessage = errorMessage,
        onOidcSignIn = onOidcSignIn
    )
}
