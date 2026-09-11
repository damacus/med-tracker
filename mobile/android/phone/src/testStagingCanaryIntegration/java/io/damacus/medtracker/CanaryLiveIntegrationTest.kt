package io.damacus.medtracker

import io.damacus.medtracker.auth.MobileAuthDiscovery
import io.damacus.medtracker.data.api.GeneratedMedTrackerApi
import io.damacus.medtracker.data.api.ApiResult
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertTrue
import org.junit.Test

class CanaryLiveIntegrationTest {
    @Test
    fun configuredMobileSessionCanDiscoverHouseholds() = runBlocking {
        val baseUrl = requiredEnvironment("MEDTRACKER_CANARY_BASE_URL")
        val token = requiredEnvironment("MEDTRACKER_CANARY_TOKEN")
        val configuration = MobileAuthDiscovery().fetch(baseUrl, "io.damacus.medtracker.staging:/oauth2redirect")
        assertTrue(configuration.clientId == "io.damacus.medtracker.staging")
        val response = GeneratedMedTrackerApi().getHouseholds(baseUrl, token)
        assertTrue("Canary household discovery failed", response is ApiResult.Success)
    }

    private fun requiredEnvironment(name: String): String =
        requireNotNull(System.getenv(name)?.takeIf(String::isNotBlank)) {
            "$name is required for the opt-in staging canary integration task"
        }
}
