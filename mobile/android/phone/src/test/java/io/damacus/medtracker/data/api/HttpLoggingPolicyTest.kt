package io.damacus.medtracker.data.api

import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.logging.HttpLoggingInterceptor
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HttpLoggingPolicyTest {
    @Test
    fun releaseLoggingIsAlwaysNone() {
        val interceptor = HttpLoggingPolicy.interceptor(
            isRelease = true,
            explicitOptIn = true
        )

        assertEquals(HttpLoggingInterceptor.Level.NONE, interceptor.level)
    }

    @Test
    fun nonReleaseLoggingRequiresOptInAndNeverExceedsBasic() {
        assertEquals(
            HttpLoggingInterceptor.Level.NONE,
            HttpLoggingPolicy.interceptor(isRelease = false, explicitOptIn = false).level
        )
        assertEquals(
            HttpLoggingInterceptor.Level.BASIC,
            HttpLoggingPolicy.interceptor(isRelease = false, explicitOptIn = true).level
        )
    }

    @Test
    fun basicLoggingOmitsCredentialsSessionHeadersAndBodies() {
        val messages = mutableListOf<String>()
        val server = MockWebServer()
        server.enqueue(MockResponse().setBody("private response body"))
        server.start()
        try {
            val client = OkHttpClient.Builder()
                .addInterceptor(
                    HttpLoggingPolicy.interceptor(
                        isRelease = false,
                        explicitOptIn = true,
                        logger = HttpLoggingInterceptor.Logger(messages::add)
                    )
                )
                .build()
            val request = Request.Builder()
                .url(server.url("/session"))
                .header("Authorization", "Bearer secret-token")
                .header("Cookie", "session=secret-session")
                .header("X-MedTracker-Portable-Passphrase", "secret-passphrase")
                .post("private request body".toRequestBody())
                .build()

            client.newCall(request).execute().close()

            val output = messages.joinToString("\n")
            assertTrue(output.contains("POST"))
            assertFalse(output.contains("secret-token"))
            assertFalse(output.contains("secret-session"))
            assertFalse(output.contains("secret-passphrase"))
            assertFalse(output.contains("private request body"))
            assertFalse(output.contains("private response body"))
        } finally {
            server.shutdown()
        }
    }

    @Test
    fun configuredSensitiveHeadersAreRedactedWhenInspectedAtHeadersLevel() {
        val messages = mutableListOf<String>()
        val server = MockWebServer()
        server.enqueue(
            MockResponse()
                .setHeader("Set-Cookie", "session=response-secret")
                .setBody("response")
        )
        server.start()
        try {
            val interceptor = HttpLoggingPolicy.interceptor(
                isRelease = false,
                explicitOptIn = true,
                logger = HttpLoggingInterceptor.Logger(messages::add)
            ).apply { level = HttpLoggingInterceptor.Level.HEADERS }
            val client = OkHttpClient.Builder().addInterceptor(interceptor).build()
            val request = Request.Builder()
                .url(server.url("/redaction"))
                .header("Authorization", "Bearer authorization-secret")
                .header("Cookie", "session=request-secret")
                .header("X-MedTracker-Portable-Passphrase", "portable-secret")
                .build()

            client.newCall(request).execute().close()

            val output = messages.joinToString("\n")
            listOf("Authorization", "Cookie", "Set-Cookie", "X-MedTracker-Portable-Passphrase").forEach {
                assertTrue(output.contains("$it: ██"))
            }
            listOf("authorization-secret", "request-secret", "response-secret", "portable-secret").forEach {
                assertFalse(output.contains(it))
            }
        } finally {
            server.shutdown()
        }
    }
}
