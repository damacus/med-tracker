package io.damacus.medtracker.auth

import net.openid.appauth.AuthorizationRequest
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LocalDevConnectionBuilderTest {

    @Test
    fun allowsLocalDevHttpHostsOnlyWhenEnabled() {
        val debugBuilder = LocalDevConnectionBuilder(allowLocalHttp = true)

        assertTrue(debugBuilder.isAllowedLocalHttpUrl("http://10.0.2.2:5150/token"))
        assertTrue(debugBuilder.isAllowedLocalHttpUrl("http://localhost:5150/token"))
        assertTrue(debugBuilder.isAllowedLocalHttpUrl("http://127.0.0.1:5150/token"))
        assertTrue(debugBuilder.isAllowedLocalHttpUrl("http://[::1]:5150/token"))
        assertFalse(debugBuilder.isAllowedLocalHttpUrl("http://example.com/token"))
        assertFalse(debugBuilder.isAllowedLocalHttpUrl("http://10.0.2.3:5150/token"))
        assertFalse(debugBuilder.isAllowedLocalHttpUrl("https://10.0.2.2:5150/token"))
    }

    @Test
    fun rejectsAllHttpHostsWhenDisabled() {
        val releaseBuilder = LocalDevConnectionBuilder(allowLocalHttp = false)

        assertFalse(releaseBuilder.isAllowedLocalHttpUrl("http://10.0.2.2:5150/token"))
        assertFalse(releaseBuilder.isAllowedLocalHttpUrl("http://localhost:5150/token"))
        assertFalse(releaseBuilder.isAllowedLocalHttpUrl("http://127.0.0.1:5150/token"))
        assertFalse(releaseBuilder.isAllowedLocalHttpUrl("http://[::1]:5150/token"))
        assertFalse(releaseBuilder.isAllowedLocalHttpUrl("http://example.com/token"))

        assertTrue(runCatching { releaseBuilder.openLocalHttpConnection("http://127.0.0.1:5150/token") }.isFailure)
    }

    @Test
    fun opensHttpConnectionToLocalMockServerWhenEnabledAndRejectsRemoteHttp() {
        val debugBuilder = LocalDevConnectionBuilder(allowLocalHttp = true)

        assertTrue(runCatching { debugBuilder.openLocalHttpConnection("http://example.com/token") }.isFailure)

        MockWebServer().use { server ->
            server.start()
            server.enqueue(MockResponse().setResponseCode(200).setBody("""{"ok":true}"""))
            val tokenUrl = server.url("/token").toString()

            val connection = debugBuilder.openLocalHttpConnection(tokenUrl)
            try {
                assertFalse(connection.instanceFollowRedirects)
                assertEquals(200, connection.responseCode)
            } finally {
                connection.disconnect()
            }
        }
    }

    @Test
    fun appAuthQueryResponseModeConstantMatchesExpectedValue() {
        assertEquals("query", AuthorizationRequest.ResponseMode.QUERY)
    }
}
