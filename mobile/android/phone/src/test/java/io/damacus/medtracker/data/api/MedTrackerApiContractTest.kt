package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.model.HouseholdSelection
import io.damacus.medtracker.data.model.RecordDosePayload
import io.medtracker.client.apis.AccountApi
import io.medtracker.client.infrastructure.ClientException
import io.medtracker.client.infrastructure.Serializer
import io.medtracker.client.models.PersonalScheduleAttributes
import io.medtracker.client.models.UpdatePersonalScheduleRequest
import java.util.UUID
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

    @Test fun generatedPersonalPlanEditEmitsOnlySchedule() {
        assertPersonalPlanWire(
            """{"schedule":{"dose_amount":"3","origin_time_zone":"Europe/London"}}""",
            UpdatePersonalScheduleRequest(schedule = PersonalScheduleAttributes(doseAmount = "3", originTimeZone = "Europe/London"))
        )
    }

    @Test fun generatedPersonalPlanLifecycleEmitsOnlyAction() {
        for (action in listOf(UpdatePersonalScheduleRequest.Action.pause, UpdatePersonalScheduleRequest.Action.resume, UpdatePersonalScheduleRequest.Action.retire)) {
            assertPersonalPlanWire("""{"action":"${action.value}"}""", UpdatePersonalScheduleRequest(action = action))
        }
    }

    private fun assertPersonalPlanWire(body: String, input: UpdatePersonalScheduleRequest) {
        val adapter = Serializer.moshi.adapter(UpdatePersonalScheduleRequest::class.java)
        assertEquals(input, adapter.fromJson(body))
        server.enqueue(MockResponse().setResponseCode(401).setBody("{\"error\":{}}"))
        val client = AccountApi(server.url("/api/v1").toString(), OkHttpClient())
        try {
            client.updatePersonalSchedule(UUID.fromString("00000000-0000-4000-8000-000000000004"), "revision-seven", input)
            throw AssertionError("Expected the stub authentication rejection")
        } catch (expected: ClientException) {
            assertEquals(401, expected.statusCode)
        }
        val request = server.takeRequest()
        assertEquals("PATCH", request.method)
        assertEquals("/api/v1/auth/schedules/00000000-0000-4000-8000-000000000004", request.path)
        assertEquals("revision-seven", request.getHeader("If-Match"))
        val json = Serializer.moshi.adapter(Any::class.java)
        assertEquals(json.fromJson(body), json.fromJson(request.body.readUtf8()))
    }

    @Test fun generatedAdapterListsHouseholdsWithTheAccountToken() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(200).setHeader("Content-Type", "application/json").setBody("""
            {"account_id":7,"data":[{"id":42,"slug":"summer-house","name":"Summer house","role":"member","membership_id":7}]}
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

    @Test fun getMeCallsCurrentProfileEndpointAndMapsUserDto() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(200).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"id":7,"email_address":"alex@example.com","membership_role":"owner","active":true,"person":{"id":11,"portable_id":"00000000-0000-0000-0000-000000000011","name":"Alex Care","email":"alex@example.com","date_of_birth":"1990-01-01","person_type":"adult","age":36,"has_capacity":true,"location_ids":[],"location_portable_ids":[],"notification_preference_id":null,"notification_preference_portable_id":null,"updated_at":"2026-03-30T10:00:00Z"},"account":{"id":7,"email":"alex@example.com","status":"verified"}}}
        """.trimIndent()))

        val result = api.getMe(server.url("/").toString(), "access-token", 42L)
        assertTrue(result is ApiResult.Success)
        val user = (result as ApiResult.Success).data
        assertEquals(7L, user.id)
        assertEquals("alex@example.com", user.emailAddress)
        assertEquals("Alex Care", user.name)
        assertEquals("owner", user.role)

        val request = server.takeRequest()
        assertEquals("GET", request.method)
        assertEquals("/api/v1/households/42/me", request.path)
        assertEquals("Bearer access-token", request.getHeader("Authorization"))
    }

    @Test fun createMedicationSendsDoseUnitAndLocationId() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(201).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"id":15,"portable_id":"00000000-0000-0000-0000-000000000015","name":"Ibuprofen","display_name":"Ibuprofen","category":"Analgesic","description":null,"dose_amount":"200","dose_unit":"mg","current_supply":"24.00","reorder_threshold":"6.00","reorder_status":null,"location_id":5,"location_portable_id":null,"updated_at":"2026-03-30T10:00:00Z","low_stock":false,"out_of_stock":false,"days_until_low_stock":null,"days_until_out_of_stock":null}}
        """.trimIndent()))

        val result = api.createMedication(
            baseUrl = server.url("/").toString(),
            accessToken = "access-token",
            householdId = 42L,
            locationId = 5,
            request = io.damacus.medtracker.data.model.CreateMedicationPayload(
                name = "Ibuprofen",
                category = "Analgesic",
                doseAmount = 200.0,
                doseUnit = "mg",
                currentSupply = 24.0,
                reorderThreshold = 6.0
            )
        )

        assertTrue(result is ApiResult.Success)
        val request = server.takeRequest()
        val body = request.body.readUtf8()
        assertTrue("Expected dose_unit in payload: $body", body.contains("\"dose_unit\":\"mg\""))
        assertTrue("Expected location_id in payload: $body", body.contains("\"location_id\":5"))
        assertTrue("Expected category in payload: $body", body.contains("\"category\":\"Analgesic\""))
    }

    @Test fun createScheduleReturnsValidationErrorOnInvalidStartDateInsteadOfThrowing() = runBlocking {
        val result = api.createSchedule(
            baseUrl = server.url("/").toString(),
            accessToken = "access-token",
            householdId = 42L,
            request = io.damacus.medtracker.data.model.CreateSchedulePayload(
                personId = 1L,
                medicationId = 2L,
                doseAmount = 1.0,
                doseUnit = "tablet",
                frequency = "daily",
                startDate = "not-a-date"
            )
        )

        assertTrue(result is ApiResult.Error)
        assertEquals("invalid_date", (result as ApiResult.Error).code)
        assertEquals(0, server.requestCount)
    }

    @Test fun doseOccurrenceEndpointsListNotTakenAndReopen() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(200).setHeader("Content-Type", "application/json").setBody("""
            {"data":[{"key":"occ-1","source_type":"schedule","source_id":7,"source_portable_id":"00000000-0000-0000-0000-000000000007","window_starts_on":"2026-03-30","window_ends_on":"2026-03-30","position":1,"scheduled_at":"2026-03-30T08:00:00Z","outcome":"open","expected":true,"due":true,"reason":null,"note":null,"resolved_at":null,"medication_take_id":null,"etag":"etag-1"}]}
        """.trimIndent()))
        server.enqueue(MockResponse().setResponseCode(200).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"key":"occ-1","source_type":"schedule","source_id":7,"source_portable_id":"00000000-0000-0000-0000-000000000007","window_starts_on":"2026-03-30","window_ends_on":"2026-03-30","position":1,"scheduled_at":"2026-03-30T08:00:00Z","outcome":"not_taken","expected":true,"due":false,"reason":"refused","note":"felt unwell","resolved_at":"2026-03-30T08:10:00Z","medication_take_id":null,"etag":"etag-2"}}
        """.trimIndent()))
        server.enqueue(MockResponse().setResponseCode(200).setHeader("Content-Type", "application/json").setBody("""
            {"data":{"key":"occ-1","source_type":"schedule","source_id":7,"source_portable_id":"00000000-0000-0000-0000-000000000007","window_starts_on":"2026-03-30","window_ends_on":"2026-03-30","position":1,"scheduled_at":"2026-03-30T08:00:00Z","outcome":"open","expected":true,"due":true,"reason":null,"note":null,"resolved_at":null,"medication_take_id":null,"etag":"etag-3"}}
        """.trimIndent()))

        val baseUrl = server.url("/").toString()
        val listRes = api.getScheduleDoseOccurrences(baseUrl, "access-token", 42L, 7L, "2026-03-30", "2026-03-30")
        assertTrue(listRes is ApiResult.Success)
        val occ = (listRes as ApiResult.Success).data.single()
        assertEquals("occ-1", occ.key)
        assertEquals("open", occ.outcome)
        assertTrue(occ.due)

        val notTakenRes = api.recordScheduleDoseNotTaken(baseUrl, "access-token", 42L, 7L, "occ-1", "refused", "felt unwell")
        assertTrue(notTakenRes is ApiResult.Success)
        assertEquals("not_taken", (notTakenRes as ApiResult.Success).data.outcome)

        val reopenRes = api.reopenScheduleDoseOccurrence(baseUrl, "access-token", 42L, 7L, "occ-1", "etag-2")
        assertTrue(reopenRes is ApiResult.Success)
        assertEquals("open", (reopenRes as ApiResult.Success).data.outcome)
    }
}
