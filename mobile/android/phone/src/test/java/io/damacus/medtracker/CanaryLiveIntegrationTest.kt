package io.damacus.medtracker

import io.damacus.medtracker.data.api.ApiResult
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.model.LoginRequest
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Ignore
import org.junit.Test

@Ignore("Live canary verification is not a hermetic unit test")
class CanaryLiveIntegrationTest {

    @Test
    fun liveCanaryServer_loginOwnerAccount() = runBlocking {
        val client = GeneratedMedTrackerApi()
        val result = client.login(
            baseUrl = "https://med-tracker-canary.damacus.io/",
            request = LoginRequest(
                email = "demo.owner@example.com",
                password = "password",
                deviceName = "Android Unit Test"
            )
        )

        assertTrue("Expected ApiResult.Success against live canary but was: $result", result is ApiResult.Success)
        val session = (result as ApiResult.Success).data
        assertTrue("Expected non-empty access token", session.accessToken.isNotBlank())
        assertTrue("Expected non-empty refresh token", session.refreshToken.isNotBlank())
        assertNotNull(session.me)
        assertEquals("demo.owner@example.com", session.me?.emailAddress)
    }

    @Test
    fun liveCanaryServer_loginCarerAccount() = runBlocking {
        val client = GeneratedMedTrackerApi()
        val result = client.login(
            baseUrl = "https://med-tracker-canary.damacus.io/",
            request = LoginRequest(
                email = "demo.carer@example.com",
                password = "password",
                deviceName = "Android Unit Test"
            )
        )

        assertTrue("Expected ApiResult.Success for carer but was: $result", result is ApiResult.Success)
        val session = (result as ApiResult.Success).data
        assertEquals("demo.carer@example.com", session.me?.emailAddress)
    }
}
