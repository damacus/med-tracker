package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.CredentialStore
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.model.SessionPayload
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class MobileSessionTokenInterceptorTest {
    private fun sessions() = SessionManager(object : CredentialStore {
        override fun read(): String? = null
        override fun write(value: String) {}
        override fun clear() {}
    })

    @Test fun refreshesBeforeSendingWithoutChangingTheSessionRevision() {
        MockWebServer().use { server ->
            server.start()
            server.enqueue(MockResponse().setBody("{}"))
            val sessions = sessions()
            sessions.saveSession(SessionPayload("old", refreshToken = "refresh", oauthState = "state"), server.url("/").toString())
            val revision = sessions.sessionState.value.revision
            var contextChanges = 0
            val observation = sessions.observeSession { contextChanges += 1 }
            val client = OkHttpClient.Builder().addInterceptor(MobileSessionTokenInterceptor(sessions) {
                RefreshedMobileTokens("new", "rotated", "new-state", "2027-01-01T00:00:00Z")
            }).build()

            client.newCall(Request.Builder().url(server.url("/api/v1/auth/households")).header("Authorization", "Bearer old").build()).execute().close()

            assertEquals("Bearer new", server.takeRequest().getHeader("Authorization"))
            assertEquals(revision, sessions.sessionState.value.revision)
            assertEquals("rotated", sessions.sessionState.value.sessionPayload?.refreshToken)
            assertEquals(1, contextChanges)
            observation.close()
        }
    }

    @Test fun doesNotSendAnExistingCredentialToAnotherInstanceOrOverwriteANewerLogin() {
        MockWebServer().use { server ->
            server.start()
            val sessions = sessions()
            sessions.saveSession(SessionPayload("old", refreshToken = "refresh", oauthState = "state"), "https://one.example/")
            val client = OkHttpClient.Builder().addInterceptor(MobileSessionTokenInterceptor(sessions) {
                error("Must not refresh for another instance")
            }).build()
            assertTrue(runCatching {
                client.newCall(Request.Builder().url(server.url("/")).header("Authorization", "Bearer old").build()).execute()
            }.isFailure)
            assertEquals(0, server.requestCount)

            sessions.saveSession(SessionPayload("old", refreshToken = "refresh", oauthState = "state"), server.url("/").toString())
            val lateClient = OkHttpClient.Builder().addInterceptor(MobileSessionTokenInterceptor(sessions) {
                sessions.saveSession(SessionPayload("other-account", refreshToken = "other-refresh"), "https://two.example/")
                RefreshedMobileTokens("late", "late-refresh", "late-state", null)
            }).build()
            assertTrue(runCatching {
                lateClient.newCall(Request.Builder().url(server.url("/")).header("Authorization", "Bearer old").build()).execute()
            }.isFailure)
            assertEquals("other-account", sessions.sessionState.value.accessToken)
            assertEquals(0, server.requestCount)
        }
    }
}
