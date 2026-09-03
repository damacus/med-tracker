package io.damacus.medtracker

import io.damacus.medtracker.auth.GeneratedPasswordAuthenticator
import io.damacus.medtracker.auth.PasswordCredentials
import io.damacus.medtracker.data.api.ApiResult
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertTrue
import org.junit.Test

class CanaryLiveIntegrationTest {
    @Test
    fun configuredAccountCanCreateASession() = runBlocking {
        val baseUrl = requiredEnvironment("MEDTRACKER_CANARY_BASE_URL")
        val email = requiredEnvironment("MEDTRACKER_CANARY_EMAIL")
        val password = requiredEnvironment("MEDTRACKER_CANARY_PASSWORD")
        val response = GeneratedPasswordAuthenticator().authenticate(
            baseUrl,
            PasswordCredentials(
                email = email,
                password = password,
                deviceName = "Android Staging Canary Integration"
            )
        )

        when (response) {
            is ApiResult.Success -> {
                assertTrue(response.data.accessToken.isNotBlank())
                assertTrue(response.data.refreshToken.isNotBlank())
            }
            else -> throw AssertionError("Canary authentication failed: $response")
        }
    }

    private fun requiredEnvironment(name: String): String =
        requireNotNull(System.getenv(name)?.takeIf(String::isNotBlank)) {
            "$name is required for the opt-in staging canary integration task"
        }
}
