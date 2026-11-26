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

    @Test fun `server paused schedules and assignments map as inactive with a resumable period`() = runBlocking {
        MockWebServer().use { server ->
            server.start()
            val meta = """"meta":{"page":1,"per_page":100,"total_count":1}"""
            repeat(2) { server.enqueue(MockResponse().setBody("""{"data":[],$meta}""")) }
            val common = """"id":1,"portable_id":"$sourceId","person_id":1,"person_portable_id":"$sourceId","medication_id":1,"medication_portable_id":"$sourceId","dose_amount":"1","dose_unit":"tablet","active":false,"paused":true,"can_manage":false,"dose_cycle":"daily","updated_at":"2026-09-08T12:00:00Z","current_pause_period":$period"""
            server.enqueue(MockResponse().setBody("""{"data":[{$common,"frequency":"daily","start_date":"2026-09-01","end_date":"2026-10-01","schedule_type":"daily","schedule_config":{}}],$meta}"""))
            server.enqueue(MockResponse().setBody("""{"data":[{$common,"administration_kind":"as_needed"}],$meta}"""))
            val gateway = GeneratedMedicationPauseGateway(GeneratedMedTrackerApi(OkHttpClient()))
            val session = AppSession(server.url("/").toString(), SessionPayload("token", refreshToken = "refresh", household = HouseholdDto(42)))

            val result = gateway.sources(session)

            assertTrue(result.toString(), result is ApiResult.Success)
            val sources = (result as ApiResult.Success).data
            assertEquals(listOf("schedule", "person_medication"), sources.map { it.type })
            sources.forEach {
                assertTrue(it.paused)
                assertFalse(it.active)
                assertEquals(periodId, it.currentPauseId)
                assertFalse(it.canManage)
            }
        }
    }

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

    @Test fun `operations handle missing household safely without crashing`() = runBlocking {
        val gateway = GeneratedMedicationPauseGateway(GeneratedMedTrackerApi(OkHttpClient()))
        val sessionWithoutHousehold = AppSession(
            "http://127.0.0.1:5150/",
            SessionPayload(accessToken = "token", refreshToken = "refresh", household = null)
        )
        val source = PauseSource("person_medication", sourceId, 1, "PRN", false)

        val sourcesResult = gateway.sources(sessionWithoutHousehold)
        assertTrue("Expected empty success for sources without household, got $sourcesResult", sourcesResult is ApiResult.Success)
        assertTrue((sourcesResult as ApiResult.Success).data.isEmpty())

        val historyResult = gateway.history(sessionWithoutHousehold, source)
        assertTrue("Expected ApiResult.Error for history without household, got $historyResult", historyResult is ApiResult.Error)

        val pauseResult = gateway.pause(sessionWithoutHousehold, source, PauseReason.OTHER, "", "req-1")
        assertTrue("Expected ApiResult.Error for pause without household, got $pauseResult", pauseResult is ApiResult.Error)

        val resumeResult = gateway.resume(sessionWithoutHousehold, periodId)
        assertTrue("Expected ApiResult.Error for resume without household, got $resumeResult", resumeResult is ApiResult.Error)
    }

    @Test fun `operations handle missing access token safely without crashing`() = runBlocking {
        val gateway = GeneratedMedicationPauseGateway(GeneratedMedTrackerApi(OkHttpClient()))
        val loggedOutSession = AppSession("http://127.0.0.1:5150/", null)
        val source = PauseSource("person_medication", sourceId, 1, "PRN", false)

        val supportedResult = gateway.supported(loggedOutSession)
        assertTrue("Expected ApiResult.Success(false) when logged out, got $supportedResult", supportedResult is ApiResult.Success && !supportedResult.data)

        val sourcesResult = gateway.sources(loggedOutSession)
        assertTrue("Expected empty success for sources when logged out, got $sourcesResult", sourcesResult is ApiResult.Success)
        assertTrue((sourcesResult as ApiResult.Success).data.isEmpty())

        val historyResult = gateway.history(loggedOutSession, source)
        assertTrue(historyResult is ApiResult.Error)

        val pauseResult = gateway.pause(loggedOutSession, source, PauseReason.OTHER, "", "req-2")
        assertTrue(pauseResult is ApiResult.Error)

        val resumeResult = gateway.resume(loggedOutSession, periodId)
        assertTrue(resumeResult is ApiResult.Error)
    }
}
