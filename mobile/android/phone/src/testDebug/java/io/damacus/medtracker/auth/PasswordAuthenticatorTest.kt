package io.damacus.medtracker.auth

import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.model.AuthenticationResult
import kotlinx.coroutines.runBlocking
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PasswordAuthenticatorTest {
    @Test fun applicationBoundaryUsesTheGeneratedPasswordOperationAndMapsHttpErrors() = runBlocking {
        MockWebServer().use { server ->
            server.start()
            server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}"))
            val authenticator: PasswordAuthenticator = GeneratedPasswordAuthenticator(OkHttpClient())

            val result = authenticator.authenticate(
                server.url("/").toString(),
                PasswordCredentials("person@example.test", "test-secret", "Debug test")
            )

            when (result) {
                is ApiResult.Error -> {
                    assertEquals("http_401", result.code)
                    assertEquals(401, result.statusCode)
                }
                else -> throw AssertionError("Expected an application-owned HTTP error")
            }
            val request = server.takeRequest()
            assertEquals("POST", request.method)
            assertEquals("/api/v1/auth/login", request.path)
            val body = request.body.readUtf8()
            assertTrue(body.contains("\"email\":\"person@example.test\""))
            assertTrue(body.contains("\"password\":\"test-secret\""))
            assertTrue(body.contains("\"device_name\":\"Debug test\""))
        }
    }

    @Test fun `password sign in returns household selection when required`() = runBlocking {
        MockWebServer().use { server ->
            server.start()
            server.enqueue(MockResponse().setResponseCode(202).setHeader("Content-Type", "application/json").setBody("""
                {"data":{"status":"household_selection_required","selection_token":"selection-token","selection_expires_at":"2026-03-30T10:05:00Z","households":[{"id":42,"slug":"summer-house","name":"Summer house","role":"member","membership_id":7}]}}
            """.trimIndent()))
            val authenticator = GeneratedPasswordAuthenticator(OkHttpClient())

            val result = authenticator.authenticate(
                server.url("/").toString(),
                PasswordCredentials("carer@example.test", "password", "Android")
            )

            assertTrue(result is ApiResult.Success)
            val selection = (result as ApiResult.Success).data as AuthenticationResult.HouseholdSelection
            assertEquals("selection-token", selection.selectionToken)
            assertEquals("Summer house", selection.households.single().name)
            assertEquals("/api/v1/auth/login", server.takeRequest().path)
        }
    }
}
