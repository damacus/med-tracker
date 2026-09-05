package io.damacus.medtracker.auth

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import io.damacus.medtracker.BuildConfig
import io.damacus.medtracker.ui.MainViewModel

@Composable
fun PasswordAuthRoute(
    buildLabel: String,
    isLoading: Boolean,
    errorMessage: String?,
    viewModel: MainViewModel
) {
    var serverUrl by remember { mutableStateOf(BuildConfig.SERVER_URL) }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    val authenticator = remember { GeneratedPasswordAuthenticator() }

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text("$buildLabel sign in")
        OutlinedTextField(
            value = serverUrl,
            onValueChange = { serverUrl = it },
            label = { Text("Server URL") },
            modifier = Modifier.fillMaxWidth()
        )
        OutlinedTextField(
            value = email,
            onValueChange = { email = it },
            label = { Text("Email") },
            modifier = Modifier.fillMaxWidth()
        )
        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Password") },
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth()
        )
        errorMessage?.let { Text(it) }
        Button(
            onClick = {
                viewModel.authenticate(serverUrl) {
                    authenticator.authenticate(
                        serverUrl,
                        PasswordCredentials(email, password, viewModel.deviceName())
                    )
                }
            },
            enabled = !isLoading && email.isNotBlank() && password.isNotBlank(),
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Sign in to $buildLabel")
        }
    }
}
