package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.model.RecordDosePayload
import kotlinx.coroutines.runBlocking
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

class ApiErrorContractTest {
    private lateinit var server: MockWebServer
    private lateinit var api: MedTrackerApi

    @Before fun setUp() {
        server = MockWebServer()
        server.start()
        api = GeneratedMedTrackerApi(OkHttpClient())
    }

    @After fun tearDown() = server.shutdown()

    @Test fun doseConflictRetainsTheServerCodeAndExplanation() = runBlocking {
        enqueueError(409, "conflict", "client_uuid was already used for a different dose")

        val result = recordDose() as ApiResult.Error

        assertEquals(409, result.statusCode)
        assertEquals("conflict", result.code)
        assertEquals("client_uuid was already used for a different dose", result.message)
        assertEquals("request-1", result.requestId)
        assertEquals(1, server.requestCount)
    }

    @Test fun invalidDoseRemainsAValidationFailure() = runBlocking {
        enqueueError(422, "unprocessable_content", "Dose amount must be positive.")

        val result = recordDose() as ApiResult.Error

        assertEquals(422, result.statusCode)
        assertEquals("unprocessable_content", result.code)
        assertEquals("Dose amount must be positive.", result.message)
        assertEquals(1, server.requestCount)
    }

    @Test fun deniedRequestsRetainTheirStatusAndExplanation() = runBlocking {
        for ((status, code) in listOf(401 to "unauthorized", 403 to "forbidden", 404 to "not_found")) {
            enqueueError(status, code, "The request was denied.")

            val result = api.getPeople(server.url("/").toString(), "account-token", 42) as ApiResult.Error

            assertEquals(status, result.statusCode)
            assertEquals(code, result.code)
            assertEquals("The request was denied.", result.message)
        }
    }

    @Test fun serverFailureRetainsTheSafeServerExplanation() = runBlocking {
        enqueueError(503, "temporarily_unavailable", "Please try again later.")

        val result = recordDose() as ApiResult.Error

        assertEquals(503, result.statusCode)
        assertEquals("temporarily_unavailable", result.code)
        assertEquals("Please try again later.", result.message)
        assertEquals("request-1", result.requestId)
    }

    @Test fun invalidErrorBodiesStayHttpFailuresWithoutExposingTheirContents() = runBlocking {
        val bodies = listOf(
            "",
            "<html>private proxy diagnostics</html>",
            "{",
            "null",
            "{}",
            "{\"error\":null}",
            "{\"error\":{\"code\":\"forbidden\"}}",
            "{\"error\":{\"code\":[],\"message\":\"private diagnostics\",\"request_id\":\"request-1\"}}",
            "{\"error\":{\"code\":\"forbidden\",\"message\":\"private diagnostics\"}}",
            "{\"error\":{\"code\":\"forbidden\",\"message\":\"private diagnostics\",\"request_id\":null}}",
            "{\"error\":{\"code\":\"forbidden\",\"message\":\"private diagnostics\",\"request_id\":\" \"}}",
            "{\"error\":{\"code\":\"\",\"message\":\"private diagnostics\",\"request_id\":\"request-1\"}}",
            "{\"error\":{\"code\":\"forbidden\",\"message\":\" \",\"request_id\":\"request-1\"}}"
        )
        for (status in listOf(422, 502)) {
            for (body in bodies) {
                server.enqueue(MockResponse().setStatus("HTTP/1.1 $status private proxy diagnostics").setBody(body))

                val result = recordDose() as ApiResult.Error

                assertEquals(status, result.statusCode)
                assertEquals("http_$status", result.code)
                assertEquals("Request failed (HTTP $status).", result.message)
                assertNull(result.requestId)
            }
        }
    }

    @Test fun fieldValidationAcceptsTheLocoEnvelopeWithAdditionalMetadata() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(422).setBody("""
            {"error":{"code":"validation_failed","message":"Validation failed","request_id":"request-1","errors":{"dose_amount":["must be a string"]},"future_metadata":true}}
        """.trimIndent()))

        val result = recordDose() as ApiResult.Error

        assertEquals(422, result.statusCode)
        assertEquals("validation_failed", result.code)
        assertEquals("Validation failed", result.message)
    }

    private fun enqueueError(status: Int, code: String, message: String) {
        server.enqueue(MockResponse().setResponseCode(status)
            .setHeader("Content-Type", "application/json")
            .setBody("""{"error":{"code":"$code","message":"$message","request_id":"request-1"}}"""))
    }

    private suspend fun recordDose() = api.recordDose(
        server.url("/").toString(), "account-token", 42,
        RecordDosePayload("00000000-0000-0000-0000-000000000001", "schedule", "7", "2026-10-24T12:00:00Z")
    )
}
