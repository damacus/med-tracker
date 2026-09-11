package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.model.HouseholdSelection
import io.damacus.medtracker.data.model.RecordDosePayload
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

    @Test fun generatedAdapterListsHouseholdsWithTheAccountToken() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(200).setHeader("Content-Type", "application/json").setBody("""
            {"data":[{"id":42,"slug":"summer-house","name":"Summer house","role":"member","membership_id":7}]}
        """.trimIndent()))
        val result = api.getHouseholds(server.url("/").toString(), "account-token")
        assertTrue(result is ApiResult.Success)
        assertEquals("Summer house", (result as ApiResult.Success).data.single().name)
        val request = server.takeRequest()
        assertEquals("/api/v1/auth/households", request.path)
        assertEquals("Bearer account-token", request.getHeader("Authorization"))
    }

    @Test fun generatedAdapterUsesContractOperationsForEveryActivePhoneRequest() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(204))
        repeat(5) { server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}")) }
        val baseUrl = server.url("/").toString()

        assertTrue(api.logout(baseUrl, "access-token") is ApiResult.Success)
        assertTrue(api.getPeople(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getMedications(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getSchedules(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.getMedicationTakes(baseUrl, "access-token", 42) is ApiResult.Error)
        assertTrue(api.recordDose(baseUrl, "access-token", 42, RecordDosePayload("00000000-0000-0000-0000-000000000001", "schedule", "7", "2026-03-30T12:00:00Z")) is ApiResult.Error)

        val requests = List(6) { server.takeRequest() }
        val paths = requests.map { it.path.orEmpty().substringBefore('?') }
        assertEquals(listOf(
            "/api/v1/auth/logout",
            "/api/v1/households/42/people", "/api/v1/households/42/medications", "/api/v1/households/42/schedules",
            "/api/v1/households/42/medication_takes", "/api/v1/households/42/medication_takes"
        ), paths)
        assertEquals("Bearer access-token", requests[0].getHeader("Authorization"))
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
