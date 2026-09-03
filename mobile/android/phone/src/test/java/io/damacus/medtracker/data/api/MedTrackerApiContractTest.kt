package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.model.OidcExchangeRequest
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RefreshRequest
import kotlinx.coroutines.runBlocking
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class MedTrackerApiContractTest {
    private lateinit var server: MockWebServer
    private lateinit var api: MedTrackerApi

    @Before fun setUp() {
        server = MockWebServer()
        server.start()
        api = GeneratedMedTrackerApi(OkHttpClient())
    }

    @After fun tearDown() = server.shutdown()

    @Test fun generatedAdapterDecodesCurrentOidcPayloadAndUnknownEnum() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(201).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"access_token":"access-token","access_token_expires_at":"2026-03-30T10:00:00Z","refresh_token":"refresh-token","refresh_token_expires_at":"2026-04-30T10:00:00Z","household":{"id":42,"slug":"summer-house","name":"Summer house"},"me":{"id":7,"email_address":"carer@example.test","membership_role":"carer","active":true,"person":{"id":9,"portable_id":"00000000-0000-0000-0000-000000000009","updated_at":"2026-03-30T10:00:00Z","name":"Care Person","email":"carer@example.test","date_of_birth":"1980-01-01","person_type":"adult","has_capacity":true,"age":46,"location_ids":[],"location_portable_ids":[],"notification_preference_id":null,"notification_preference_portable_id":null},"account":{"id":7,"email":"carer@example.test","status":"verified"}}}}
        """.trimIndent()))

        val result = api.exchangeOidc(
            server.url("/").toString(),
            OidcExchangeRequest("id-token", "nonce", "verifier")
        )

        assertTrue(result is ApiResult.Success)
        val session = (result as ApiResult.Success).data
        assertEquals("access-token", session.accessToken)
        assertEquals("Summer house", session.household?.name)
        assertEquals("Care Person", session.me?.name)
        assertEquals("unknown_default_open_api", session.me?.role)
    }

    @Test fun generatedAdapterUsesContractOperationsForEveryActivePhoneRequest() = runBlocking {
        repeat(2) { server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}")) }
        server.enqueue(MockResponse().setResponseCode(204))
        repeat(5) { server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}")) }
        val baseUrl = server.url("/").toString()

        assertTrue(api.exchangeOidc(baseUrl, OidcExchangeRequest("id-token", "nonce", "verifier")) is ApiResult.Error)
        assertTrue(api.refresh(baseUrl, RefreshRequest("refresh-token")) is ApiResult.Error)
        assertTrue(api.logout(baseUrl, "access-token") is ApiResult.Success)
        assertTrue(api.getPeople(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getMedications(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getSchedules(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getMedicationTakes(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.recordDose(baseUrl, "access-token", 42, RecordDosePayload("00000000-0000-0000-0000-000000000001", "schedule", "7", "2026-03-30T12:00:00Z")) is ApiResult.Error)

        val requests = List(8) { server.takeRequest() }
        val paths = requests.map { it.path.orEmpty().substringBefore('?') }
        assertEquals(listOf(
            "/api/v1/auth/oidc_exchange", "/api/v1/auth/refresh", "/api/v1/auth/logout",
            "/api/v1/households/42/people", "/api/v1/households/42/medications", "/api/v1/households/42/schedules",
            "/api/v1/households/42/medication_takes", "/api/v1/households/42/medication_takes"
        ), paths)
        assertEquals("Bearer access-token", requests[2].getHeader("Authorization"))
    }
}
