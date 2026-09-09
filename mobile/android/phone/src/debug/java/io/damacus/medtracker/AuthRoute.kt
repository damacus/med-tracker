package io.damacus.medtracker

import androidx.compose.runtime.Composable
import io.damacus.medtracker.auth.PasswordAuthRoute
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
        PasswordAuthRoute(
            buildLabel = "Debug",
            isLoading = isLoading,
            errorMessage = errorMessage,
            viewModel = viewModel
        )
    } else {
        HouseholdSelectionRoute(householdSelection, isLoading, errorMessage, viewModel::selectHousehold)
    }
}
