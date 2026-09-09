package io.damacus.medtracker.auth

import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.ApiResult
import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class LoginIntegrationTest {

    private lateinit var mockWebServer: MockWebServer
    private lateinit var authenticator: GeneratedPasswordAuthenticator
    private lateinit var sessionManager: SessionManager
    private lateinit var credentialStore: MemoryCredentialStore

    @Before
    fun setUp() {
        mockWebServer = MockWebServer()
        mockWebServer.start()

        authenticator = GeneratedPasswordAuthenticator()
        credentialStore = MemoryCredentialStore()
        sessionManager = SessionManager(credentialStore)
    }

    @After
    fun tearDown() {
        mockWebServer.shutdown()
    }

    @Test
    fun successfulLoginReturnsSessionPayloadAndSavesSession() = runBlocking {
        val jsonResponse = """
            {
              "data": {
                "access_token": "mock-access-token",
                "access_token_expires_at": "2026-12-31T23:59:59Z",
                "refresh_token": "mock-refresh-token",
                "refresh_token_expires_at": "2027-12-31T23:59:59Z",
                "me": {
                  "id": 1,
                  "email_address": "jane.doe@example.com",
                  "membership_role": "owner",
                  "active": true,
                  "person": {
                    "id": 1,
                    "portable_id": "00000000-0000-0000-0000-000000000001",
                    "updated_at": "2026-01-01T00:00:00Z",
                    "name": "Jane Doe",
                    "email": "jane.doe@example.com",
                    "date_of_birth": "1980-01-01",
                    "person_type": "adult",
                    "has_capacity": true,
                    "location_ids": [],
                    "location_portable_ids": []
                  },
                  "account": {
                    "id": 1,
                    "email": "jane.doe@example.com",
                    "status": "verified"
                  }
                },
                "household": {
                  "id": 42,
                  "slug": "demo-household",
                  "name": "Demo Household"
                }
              }
            }
        """.trimIndent()

        mockWebServer.enqueue(MockResponse().setResponseCode(201).setBody(jsonResponse))

        val serverUrl = mockWebServer.url("/").toString()
        val credentials = PasswordCredentials(
            email = "jane.doe@example.com",
            password = "password",
            deviceName = "TestDevice"
        )

        val result = authenticator.authenticate(serverUrl, credentials)
        assertTrue("Expected ApiResult.Success but was $result", result is ApiResult.Success)

        val payload = (result as ApiResult.Success).data
        assertEquals("mock-access-token", payload.accessToken)
        assertEquals("Jane Doe", payload.me?.name)
        assertEquals(42L, payload.household?.id)

        // Save session using SessionManager
        sessionManager.saveSession(payload, serverUrl)
        val savedSession = sessionManager.sessionState.value

        assertTrue(savedSession.isLoggedIn)
        assertEquals("mock-access-token", savedSession.accessToken)
        assertEquals("Jane Doe", savedSession.user?.name)
        assertEquals("Demo Household", savedSession.household?.name)

        // Verify request was sent to correct path with login payload
        val recordedRequest = mockWebServer.takeRequest()
        assertEquals("/api/v1/auth/login", recordedRequest.path)
        assertEquals("POST", recordedRequest.method)
        val requestBody = recordedRequest.body.readUtf8()
        assertTrue(requestBody.contains("jane.doe@example.com"))
        assertTrue(requestBody.contains("password"))
    }

    @Test
    fun loginWithNullMembershipRoleSucceedsWithoutError() = runBlocking {
        val jsonResponse = """
            {
              "data": {
                "access_token": "mock-token-null-role",
                "access_token_expires_at": "2026-12-31T23:59:59Z",
                "refresh_token": "mock-refresh",
                "refresh_token_expires_at": "2027-12-31T23:59:59Z",
                "me": {
                  "id": 2,
                  "email_address": "carer@example.com",
                  "membership_role": null,
                  "active": true,
                  "person": {
                    "id": 2,
                    "portable_id": "00000000-0000-0000-0000-000000000002",
                    "updated_at": "2026-01-01T00:00:00Z",
                    "name": "Carer User",
                    "person_type": "adult",
                    "has_capacity": true,
                    "location_ids": [],
                    "location_portable_ids": []
                  },
                  "account": {
                    "id": 2,
                    "email": "carer@example.com",
                    "status": "verified"
                  }
                },
                "household": {
                  "id": 1,
                  "slug": "default",
                  "name": "Default Household"
                }
              }
            }
        """.trimIndent()

        mockWebServer.enqueue(MockResponse().setResponseCode(201).setBody(jsonResponse))

        val serverUrl = mockWebServer.url("/").toString()
        val credentials = PasswordCredentials("carer@example.com", "password", "TestDevice")

        val result = authenticator.authenticate(serverUrl, credentials)
        assertTrue("Expected ApiResult.Success for null membership_role but was $result", result is ApiResult.Success)

        val payload = (result as ApiResult.Success).data
        assertEquals("Carer User", payload.me?.name)
        assertNull(payload.me?.role)
    }

    @Test
    fun failedLoginReturnsUnauthorizedError() = runBlocking {
        val jsonError = """
            {
              "error": {
                "code": "invalid_credentials",
                "message": "Email or password is invalid"
              }
            }
        """.trimIndent()

        mockWebServer.enqueue(MockResponse().setResponseCode(401).setBody(jsonError))

        val serverUrl = mockWebServer.url("/").toString()
        val credentials = PasswordCredentials(
            email = "invalid@example.com",
            password = "wrongpassword",
            deviceName = "TestDevice"
        )

        val result = authenticator.authenticate(serverUrl, credentials)
        assertTrue("Expected ApiResult.Error", result is ApiResult.Error)

        val error = result as ApiResult.Error
        assertEquals(401, error.statusCode)
        assertNotNull(error.message)
    }

    @Test
    fun loginWithUnprocessableContentReturnsError() = runBlocking {
        val jsonError = """
            {
              "status": 422,
              "error": "Unprocessable Content"
            }
        """.trimIndent()

        mockWebServer.enqueue(MockResponse().setResponseCode(422).setBody(jsonError))

        val serverUrl = mockWebServer.url("/").toString()
        val credentials = PasswordCredentials(
            email = "unprocessed@example.com",
            password = "password",
            deviceName = "TestDevice"
        )

        val result = authenticator.authenticate(serverUrl, credentials)
        assertTrue("Expected ApiResult.Error for 422 response", result is ApiResult.Error)

        val error = result as ApiResult.Error
        assertEquals(422, error.statusCode)
    }

    private class MemoryCredentialStore : CredentialStore {
        private var stored: String? = null
        override fun read(): String? = stored
        override fun write(value: String) { stored = value }
        override fun clear() { stored = null }
    }
}
