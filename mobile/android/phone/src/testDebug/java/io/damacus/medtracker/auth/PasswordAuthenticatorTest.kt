package io.damacus.medtracker.auth

import io.damacus.medtracker.data.api.ApiResult
import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PasswordAuthenticatorTest {
    @Test
    fun applicationBoundaryUsesTheGeneratedPasswordOperationAndMapsHttpErrors() = runBlocking {
        val server = MockWebServer()
        server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}"))
        server.start()
        try {
            val authenticator: PasswordAuthenticator = GeneratedPasswordAuthenticator()

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
        } finally {
            server.shutdown()
        }
    }
}
