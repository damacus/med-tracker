package io.damacus.medtracker.auth

import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.HttpLoggingPolicy
import io.damacus.medtracker.data.api.RequestAuthCallFactory
import io.damacus.medtracker.data.api.authenticationRequest
import io.damacus.medtracker.data.model.AuthenticationResult
import io.medtracker.client.infrastructure.ApiResponse
import io.medtracker.client.models.AuthLoginRequest
import io.medtracker.password.client.apis.AuthenticationApi
import okhttp3.Call

data class PasswordCredentials(
    val email: String,
    val password: String,
    val deviceName: String
)

interface PasswordAuthenticator {
    suspend fun authenticate(serverUrl: String, credentials: PasswordCredentials): ApiResult<AuthenticationResult>
}

class GeneratedPasswordAuthenticator(
    private val callFactory: Call.Factory = HttpLoggingPolicy.client()
) : PasswordAuthenticator {
    private val unauthenticatedCalls = RequestAuthCallFactory(callFactory)

    override suspend fun authenticate(
        serverUrl: String,
        credentials: PasswordCredentials
    ): ApiResult<AuthenticationResult> = authenticationRequest {
        MultiResponsePasswordApi(
            "${serverUrl.trimEnd('/')}/api/v1",
            unauthenticatedCalls
        ).login(
                AuthLoginRequest(
                    credentials.email,
                    credentials.password,
                    credentials.deviceName
                )
            )
    }
}

private class MultiResponsePasswordApi(basePath: String, client: Call.Factory) : AuthenticationApi(basePath, client) {
    fun login(requestBody: AuthLoginRequest): ApiResponse<Map<String, Any?>?> =
        request<AuthLoginRequest, Map<String, Any?>>(createLoginSessionRequestConfig(requestBody))
}
