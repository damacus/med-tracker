package io.damacus.medtracker.auth

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MobileAuthDiscoveryTest {
    private val callback = "io.damacus.medtracker.debug:/oauth2redirect"

    @Test
    fun discoversRegisteredClientWithoutSendingCredentials() = runTest {
        MockWebServer().use { server ->
            server.start()
            val origin = server.url("/").toString().trimEnd('/')
            server.enqueue(MockResponse().setBody(capabilities(origin)))
            server.enqueue(MockResponse().setBody(metadata(origin)))

            val result = MobileAuthDiscovery(OkHttpClient(), allowLocalHttp = true).fetch(origin, callback)

            assertEquals("mobile-test", result.clientId)
            assertEquals("$origin/authorize", result.authorizationEndpoint)
            assertEquals("$origin/token", result.tokenEndpoint)
            assertEquals("/api/v1/capabilities", server.takeRequest().path)
            val discovery = server.takeRequest()
            assertEquals("/.well-known/oauth-authorization-server", discovery.path)
            assertNull(discovery.getHeader("Authorization"))
        }
    }

    @Test
    fun rejectsAnEndpointOnAnotherInstance() = runTest {
        MockWebServer().use { server ->
            server.start()
            val origin = server.url("/").toString().trimEnd('/')
            server.enqueue(MockResponse().setBody(capabilities(origin)))
            server.enqueue(MockResponse().setBody(metadata(origin).replace("$origin/token", "https://other.example/token")))

            val result = runCatching { MobileAuthDiscovery(OkHttpClient(), allowLocalHttp = true).fetch(origin, callback) }

            assertTrue(result.isFailure)
        }
    }

    @Test
    fun rejectsUnregisteredCallbacksAndInsecureRemoteInstances() = runTest {
        MockWebServer().use { server ->
            server.start()
            val origin = server.url("/").toString().trimEnd('/')
            server.enqueue(MockResponse().setBody(capabilities(origin)))
            val discovery = MobileAuthDiscovery(OkHttpClient(), allowLocalHttp = true)

            assertTrue(runCatching { discovery.fetch(origin, "io.other:/callback") }.isFailure)
            assertTrue(runCatching { discovery.fetch("http://example.com/", callback) }.isFailure)
            assertTrue(runCatching { discovery.fetch("https://user:password@example.com/", callback) }.isFailure)
        }
    }

    private fun capabilities(origin: String) = """
        {"data":{"authentication":{"mobile_oauth":{
          "discovery_url":"$origin/.well-known/oauth-authorization-server",
          "household_binding":"account",
          "clients":[{"client_id":"mobile-test","redirect_uris":["$callback"],"scopes":["medtracker","offline_access"]}]
        }}}}
    """.trimIndent()

    private fun metadata(origin: String) = """
        {"issuer":"$origin","authorization_endpoint":"$origin/authorize","token_endpoint":"$origin/token",
         "revocation_endpoint":"$origin/revoke","grant_types_supported":["authorization_code","refresh_token"],
         "code_challenge_methods_supported":["S256"]}
    """.trimIndent()
}
