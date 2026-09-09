package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.model.OidcExchangeRequest
import io.damacus.medtracker.data.model.AuthenticationResult
import io.damacus.medtracker.data.model.HouseholdSelectionRequest
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
        val session = ((result as ApiResult.Success).data as AuthenticationResult.Session).payload
        assertEquals("access-token", session.accessToken)
        assertEquals("Summer house", session.household?.name)
        assertEquals("Care Person", session.me?.name)
        assertEquals("unknown_default_open_api", session.me?.role)
    }

    @Test fun generatedAdapterCompletesOidcHouseholdSelection() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(202).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"status":"household_selection_required","selection_token":"selection-token","selection_expires_at":"2026-03-30T10:05:00Z","households":[{"id":42,"slug":"summer-house","name":"Summer house","role":"member","membership_id":7}]}}
        """.trimIndent()))
        server.enqueue(MockResponse().setResponseCode(201).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"access_token":"access-token","access_token_expires_at":"2026-03-30T10:00:00Z","refresh_token":"refresh-token","refresh_token_expires_at":"2026-04-30T10:00:00Z","household":{"id":42,"slug":"summer-house","name":"Summer house"},"me":{"id":7,"email_address":"carer@example.test","membership_role":"member","active":true,"person":{"id":9,"portable_id":"00000000-0000-0000-0000-000000000009","updated_at":"2026-03-30T10:00:00Z","name":"Care Person","email":"carer@example.test","date_of_birth":"1980-01-01","person_type":"adult","has_capacity":true,"age":46,"location_ids":[],"location_portable_ids":[],"notification_preference_id":null,"notification_preference_portable_id":null},"account":{"id":7,"email":"carer@example.test","status":"verified"}}}}
        """.trimIndent()))
        val baseUrl = server.url("/").toString()

        val exchange = api.exchangeOidc(baseUrl, OidcExchangeRequest("id-token", "nonce", "verifier"))
        val selection = (exchange as ApiResult.Success).data as AuthenticationResult.HouseholdSelection
        assertEquals("selection-token", selection.selectionToken)
        assertEquals("Summer house", selection.households.single().name)

        val selected = api.selectHousehold(baseUrl, HouseholdSelectionRequest(selection.selectionToken, 42))
        assertEquals("access-token", (selected as ApiResult.Success).data.accessToken)
        assertEquals(
            listOf("/api/v1/auth/oidc_exchange", "/api/v1/auth/select_household"),
            List(2) { server.takeRequest().path }
        )
    }

    @Test fun generatedAdapterUsesContractOperationsForEveryActivePhoneRequest() = runBlocking {
        repeat(3) { server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}")) }
        server.enqueue(MockResponse().setResponseCode(204))
        repeat(5) { server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}")) }
        val baseUrl = server.url("/").toString()

        assertTrue(api.exchangeOidc(baseUrl, OidcExchangeRequest("id-token", "nonce", "verifier")) is ApiResult.Error)
        assertTrue(api.selectHousehold(baseUrl, HouseholdSelectionRequest("selection-token", 42)) is ApiResult.Error)
        assertTrue(api.refresh(baseUrl, RefreshRequest("refresh-token")) is ApiResult.Error)
        assertTrue(api.logout(baseUrl, "access-token") is ApiResult.Success)
        assertTrue(api.getPeople(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getMedications(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getSchedules(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getMedicationTakes(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.recordDose(baseUrl, "access-token", 42, RecordDosePayload("00000000-0000-0000-0000-000000000001", "schedule", "7", "2026-03-30T12:00:00Z")) is ApiResult.Error)

        val requests = List(9) { server.takeRequest() }
        val paths = requests.map { it.path.orEmpty().substringBefore('?') }
        assertEquals(listOf(
            "/api/v1/auth/oidc_exchange", "/api/v1/auth/select_household", "/api/v1/auth/refresh", "/api/v1/auth/logout",
            "/api/v1/households/42/people", "/api/v1/households/42/medications", "/api/v1/households/42/schedules",
            "/api/v1/households/42/medication_takes", "/api/v1/households/42/medication_takes"
        ), paths)
        assertEquals("Bearer access-token", requests[3].getHeader("Authorization"))
    }

    @Test fun generatedAdapterRecordsStockRemovalUsingStockRemovalsEndpoint() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(201).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"id":"rem-1","medication_id":"15","dosage_id":null,"quantity":"2.00","reason":"damaged","note":"damaged","submission_id":"00000000-0000-0000-0000-000000000001","previous_quantity":"10.00","remaining_quantity":"8.00","unit":"pills","created_at":"2026-03-30T10:00:00Z","actor_membership_id":"1"}}
        """.trimIndent()))
        server.enqueue(MockResponse().setResponseCode(200).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"id":15,"portable_id":"00000000-0000-0000-0000-000000000015","name":"Paracetamol","display_name":"Paracetamol","category":"Pain","description":null,"dose_amount":"500","dose_unit":"mg","current_supply":"8.00","reorder_threshold":"5.00","reorder_status":null,"location_id":1,"location_portable_id":null,"updated_at":"2026-03-30T10:00:00Z","low_stock":false,"out_of_stock":false,"days_until_low_stock":null,"days_until_out_of_stock":null}}
        """.trimIndent()))

        val baseUrl = server.url("/").toString()
        val result = api.recordStockRemoval(
            baseUrl = baseUrl,
            accessToken = "access-token",
            householdId = 42L,
            request = io.damacus.medtracker.data.model.RecordStockRemovalPayload(
                medicationId = 15L,
                quantity = 2.0,
                reason = "damaged",
                removedAt = "2026-03-30T10:00:00Z"
            )
        )

        assertTrue(result is ApiResult.Success)
        val med = (result as ApiResult.Success).data
        assertEquals(8.0, med.currentSupply)

        val stockRemovalReq = server.takeRequest()
        assertEquals("/api/v1/households/42/medications/15/stock_removals", stockRemovalReq.path)
        assertEquals("POST", stockRemovalReq.method)

        val getMedReq = server.takeRequest()
        assertEquals("/api/v1/households/42/medications/15", getMedReq.path)
        assertEquals("GET", getMedReq.method)
    }
}
