package io.damacus.medtracker.auth

import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.HttpLoggingPolicy
import io.damacus.medtracker.data.api.toSessionPayload
import io.damacus.medtracker.data.model.SessionPayload
import io.medtracker.client.infrastructure.ClientException
import io.medtracker.client.infrastructure.ServerException
import io.medtracker.client.models.AuthLoginRequest
import io.medtracker.password.client.apis.AuthenticationApi
import java.io.IOException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Call

data class PasswordCredentials(
    val email: String,
    val password: String,
    val deviceName: String
)

interface PasswordAuthenticator {
    suspend fun authenticate(serverUrl: String, credentials: PasswordCredentials): ApiResult<SessionPayload>
}

class GeneratedPasswordAuthenticator(
    private val callFactory: Call.Factory = HttpLoggingPolicy.client()
) : PasswordAuthenticator {
    override suspend fun authenticate(
        serverUrl: String,
        credentials: PasswordCredentials
    ): ApiResult<SessionPayload> = withContext(Dispatchers.IO) {
        try {
            val data = AuthenticationApi(
                "${serverUrl.trimEnd('/')}/api/v1",
                callFactory
            ).createLoginSession(
                AuthLoginRequest(
                    credentials.email,
                    credentials.password,
                    credentials.deviceName
                )
            ).data
            ApiResult.Success(data.toSessionPayload())
        } catch (error: ClientException) {
            ApiResult.Error("http_${error.statusCode}", error.message.orEmpty(), error.statusCode)
        } catch (error: ServerException) {
            ApiResult.Error("http_${error.statusCode}", error.message.orEmpty(), error.statusCode)
        } catch (error: IOException) {
            ApiResult.NetworkError(error)
        }
    }
}
