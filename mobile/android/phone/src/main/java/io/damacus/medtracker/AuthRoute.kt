package io.damacus.medtracker

import androidx.compose.runtime.Composable
import io.damacus.medtracker.auth.OidcAuthRoute
import io.damacus.medtracker.auth.HouseholdSelectionRoute
import io.damacus.medtracker.data.model.HouseholdSelection
import io.damacus.medtracker.ui.MainViewModel

@Composable
fun AuthRoute(
    isLoading: Boolean,
    errorMessage: String?,
    householdSelection: HouseholdSelection?,
    viewModel: MainViewModel,
    onOidcSignIn: (String) -> Unit
) {
    if (householdSelection == null) {
        OidcAuthRoute(
            title = "Sign in securely with MedTracker",
            isLoading = isLoading,
            errorMessage = errorMessage,
            onOidcSignIn = onOidcSignIn
        )
    } else {
        HouseholdSelectionRoute(householdSelection, isLoading, errorMessage,
            onRetry = viewModel::showHouseholds, onLogout = { viewModel.logout() }, onSelect = viewModel::selectHousehold)
    }
}
