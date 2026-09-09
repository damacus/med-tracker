package io.damacus.medtracker

import androidx.compose.runtime.Composable
import io.damacus.medtracker.auth.OidcAuthRoute
import io.damacus.medtracker.auth.HouseholdSelectionRoute
import io.damacus.medtracker.data.model.AuthenticationResult
import io.damacus.medtracker.ui.MainViewModel

@Composable
fun AuthRoute(
    isLoading: Boolean,
    errorMessage: String?,
    householdSelection: AuthenticationResult.HouseholdSelection?,
    viewModel: MainViewModel,
    onOidcSignIn: () -> Unit
) {
    if (householdSelection == null) {
        OidcAuthRoute(
            title = "Sign in securely with MedTracker",
            isLoading = isLoading,
            errorMessage = errorMessage,
            onOidcSignIn = onOidcSignIn
        )
    } else {
        HouseholdSelectionRoute(householdSelection, isLoading, errorMessage, viewModel::selectHousehold)
    }
}
