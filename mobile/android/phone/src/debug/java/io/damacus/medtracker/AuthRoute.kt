package io.damacus.medtracker

import androidx.compose.runtime.Composable
import io.damacus.medtracker.auth.PasswordAuthRoute
import io.damacus.medtracker.ui.MainViewModel

@Composable
fun AuthRoute(
    isLoading: Boolean,
    errorMessage: String?,
    viewModel: MainViewModel,
    onOidcSignIn: () -> Unit
) {
    PasswordAuthRoute(
        buildLabel = "Debug",
        isLoading = isLoading,
        errorMessage = errorMessage,
        viewModel = viewModel
    )
}
