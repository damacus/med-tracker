package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.model.*
import kotlinx.coroutines.runBlocking
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.*
import org.junit.Test

class MedicationPauseGatewayTest {
    private val sourceId = "00000000-0000-0000-0000-000000000001"
    private val periodId = "00000000-0000-0000-0000-000000000002"
    private val period = """{"id":"$periodId","portable_id":"$periodId","source_type":"person_medication","source_id":"$sourceId","reason":"other","note":null,"legacy_context":false,"started_at":"2026-09-08T12:00:00Z","ended_at":null,"recorded_by_membership_id":"member","resumed_by_membership_id":null,"recorded_by_name":"Alex","resumed_by_name":null,"created_at":"2026-09-08T12:00:00Z","updated_at":"2026-09-08T12:00:00Z"}"""

    @Test fun `pause sends portable source reason optional note and no client effective time`() = runBlocking {
        MockWebServer().use { server ->
            server.start()
            server.enqueue(MockResponse().setHeader("Content-Type", "application/json").setBody("""{"data":$period}"""))
            val gateway = GeneratedMedicationPauseGateway(GeneratedMedTrackerApi(OkHttpClient()))
            val session = AppSession(server.url("/").toString(), SessionPayload("token", refreshToken = "refresh", household = HouseholdDto(42)))
            val result = gateway.pause(session, PauseSource("person_medication", sourceId, 1, "PRN", false), PauseReason.OTHER, "", "pause-attempt")
            assertTrue(result is ApiResult.Success)
            val request = server.takeRequest()
            assertEquals("/api/v1/households/42/medication_pause_periods", request.path)
            assertEquals("Bearer token", request.getHeader("Authorization"))
            assertEquals("pause-attempt", request.getHeader("Idempotency-Key"))
            val body = request.body.readUtf8()
            assertTrue(body.contains(sourceId))
            assertTrue(body.contains("\"reason\":\"other\""))
            assertFalse(body.contains("started_at"))
            assertFalse(body.contains("ended_at"))
        }
    }

    @Test fun `history requests every page with paired portable filters`() = runBlocking {
        MockWebServer().use { server ->
            server.start()
            (1..2).forEach { page -> server.enqueue(MockResponse().setHeader("Content-Type", "application/json").setBody("""{"data":[$period],"meta":{"page":$page,"per_page":1,"total_count":2}}""")) }
            val gateway = GeneratedMedicationPauseGateway(GeneratedMedTrackerApi(OkHttpClient()))
            val session = AppSession(server.url("/").toString(), SessionPayload("token", refreshToken = "refresh", household = HouseholdDto(42)))
            val result = gateway.history(session, PauseSource("person_medication", sourceId, 1, "PRN", false))
            assertEquals(2, (result as ApiResult.Success).data.size)
            repeat(2) { index ->
                val request = server.takeRequest().requestUrl!!
                assertEquals((index + 1).toString(), request.queryParameter("page"))
                assertEquals("person_medication", request.queryParameter("source_type"))
                assertEquals(sourceId, request.queryParameter("source_id"))
            }
        }
    }
}
