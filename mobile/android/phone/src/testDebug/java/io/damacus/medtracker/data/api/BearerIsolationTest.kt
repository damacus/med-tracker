package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.model.RecordDosePayload
import io.medtracker.client.infrastructure.ApiClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.runBlocking
import okhttp3.Call
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class BearerIsolationTest {
    private val serverA = MockWebServer()
    private val serverB = MockWebServer()
    private val client = OkHttpClient()
    private val api = GeneratedMedTrackerApi(client)

    @Before fun setUp() {
        serverA.start()
        serverB.start()
    }

    @After fun tearDown() {
        serverA.shutdown()
        serverB.shutdown()
    }

    @Test fun `protected requests to different servers complete independently with only their own bearer`() = withAmbientBearer {
        val releasePeople = holdResponse(serverA, 401)
        val people = async(Dispatchers.IO) { api.getPeople(serverA.url("/").toString(), "account-a-token", 1) }
        try {
            val peopleRequest = requestFrom(serverA)
            serverB.enqueue(unauthorized())
            val medications = async(Dispatchers.IO) { api.getMedications(serverB.url("/").toString(), "account-b-token", 2) }

            val medicationsRequest = requestFrom(serverB)
            assertTrue(medications.await() is ApiResult.Error)
            assertFalse(people.isCompleted)
            assertEquals(listOf("Bearer account-a-token"), peopleRequest.headers.values("Authorization"))
            assertEquals(listOf("Bearer account-b-token"), medicationsRequest.headers.values("Authorization"))
            assertEquals("ambient-token-sentinel", ApiClient.accessToken)
        } finally {
            releasePeople.countDown()
            people.await()
        }
    }

    @Test fun `capability discovery strips ambient generated bearer`() = withAmbientBearer {
        serverB.enqueue(unauthorized())
        assertTrue(api.getCapabilities(serverB.url("/").toString()) is ApiResult.Error)
        assertNull(requestFrom(serverB).getHeader("Authorization"))
    }

    @Test fun `every protected operation replaces ambient bearer with the request credential`() = withAmbientBearer {
        repeat(6) { serverA.enqueue(unauthorized()) }
        val baseUrl = serverA.url("/").toString()

        api.logout(baseUrl, "request-token")
        api.getPeople(baseUrl, "request-token", 1)
        api.getMedications(baseUrl, "request-token", 1)
        api.getSchedules(baseUrl, "request-token", 1)
        api.getMedicationTakes(baseUrl, "request-token", 1)
        api.recordDose(baseUrl, "request-token", 1, RecordDosePayload("00000000-0000-0000-0000-000000000001", "schedule", "7", "2026-03-30T12:00:00Z"))

        repeat(6) {
            assertEquals(listOf("Bearer request-token"), requestFrom(serverA).headers.values("Authorization"))
        }
        assertEquals("ambient-token-sentinel", ApiClient.accessToken)
    }

    @Test fun `request authentication preserves the configured delegate call and cancellation`() {
        val delegatedCalls = mutableListOf<Call>()
        val delegate = Call.Factory { request -> client.newCall(request).also(delegatedCalls::add) }
        val request = Request.Builder().url(serverA.url("/")).header("Authorization", "Bearer ambient-token").build()

        val call = RequestAuthCallFactory(delegate, "request-token").newCall(request)

        assertSame(delegatedCalls.single(), call)
        assertEquals(listOf("Bearer request-token"), call.request().headers.values("Authorization"))
        call.cancel()
        assertTrue(delegatedCalls.single().isCanceled())
    }

    private fun withAmbientBearer(block: suspend CoroutineScope.() -> Unit) = runBlocking {
        val previous = ApiClient.accessToken
        ApiClient.accessToken = "ambient-token-sentinel"
        try {
            coroutineScope { block() }
        } finally {
            ApiClient.accessToken = previous
        }
    }

    private fun holdResponse(server: MockWebServer, statusCode: Int): CountDownLatch {
        val release = CountDownLatch(1)
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                check(release.await(10, TimeUnit.SECONDS)) { "Test did not release the held response" }
                return MockResponse().setResponseCode(statusCode).setBody(if (statusCode == 204) "" else "{\"error\":{}}")
            }
        }
        return release
    }

    private fun requestFrom(server: MockWebServer): RecordedRequest =
        server.takeRequest(3, TimeUnit.SECONDS) ?: throw AssertionError("Expected an independent request before releasing the old response")

    private fun unauthorized() = MockResponse().setResponseCode(401).setBody("{\"error\":{}}")
}
