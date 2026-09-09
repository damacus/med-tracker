package io.damacus.medtracker.ui.dashboard

import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.api.*
import io.damacus.medtracker.data.model.*
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.*
import org.junit.Assert.*
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class MedicationPauseControllerTest {
    private val source = PauseSource("schedule", "portable-source", 1, "Medicine", false)
    private val period = PausePeriod("period", "schedule", source.id, "other", null, "2026-09-08T12:00:00Z", null, false, "Alex", null)

    @Test fun `pause waits for server and network failure retains input without queueing`() = runTest {
        val gateway = FakeGateway()
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        controller.edit(source)
        controller.reason(PauseReason.OTHER)
        controller.note("Keep my note")
        controller.submit(); runCurrent()
        assertFalse(controller.state.value.sources.single().paused)
        assertTrue(controller.state.value.form.submitting)
        gateway.result.complete(ApiResult.NetworkError(java.io.IOException()))
        runCurrent()
        assertFalse(controller.state.value.sources.single().paused)
        assertEquals("Keep my note", controller.state.value.form.note)
        assertEquals(PauseReason.OTHER, controller.state.value.form.reason)
        assertNotNull(controller.state.value.form.error)
        assertEquals(1, gateway.requests)
        val attempt = gateway.requestIds.single()
        controller.submit(); runCurrent()
        assertEquals(listOf(attempt, attempt), gateway.requestIds)
    }

    @Test fun `confirmed pause uses server period and ignores late results for old session`() = runTest {
        val gateway = FakeGateway()
        var current = true
        var confirmed = 0
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { current }, { confirmed++ })
        controller.refresh(); runCurrent()
        controller.edit(source); controller.reason(PauseReason.OTHER); controller.submit(); runCurrent()
        current = false
        gateway.result.complete(ApiResult.Success(period)); runCurrent()
        assertFalse(controller.state.value.sources.single().paused)
        assertEquals(0, confirmed)
    }

    @Test fun `unsupported server exposes no pause controls or mutations`() = runTest {
        val gateway = FakeGateway(supported = false)
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        controller.edit(source); controller.reason(PauseReason.OTHER); controller.submit(); runCurrent()
        assertFalse(controller.state.value.supported)
        assertEquals(0, gateway.requests)
        assertEquals(0, gateway.sourceRequests)
    }

    @Test fun `a failed capability refresh retains loaded pause controls and reports the failure`() = runTest {
        val gateway = FakeGateway()
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        gateway.capability = ApiResult.NetworkError(java.io.IOException())

        controller.refresh(); runCurrent()

        assertTrue(controller.state.value.supported)
        assertEquals(listOf(source), controller.state.value.sources)
        assertEquals("Connect to the internet and try again. No change was confirmed.", controller.state.value.error)
    }

    @Test fun `resume changes source only after server confirmation`() = runTest {
        val gateway = FakeGateway(paused = true)
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        controller.resume(controller.state.value.sources.single()); runCurrent()
        assertTrue(controller.state.value.sources.single().paused)
        gateway.paused = false
        gateway.result.complete(ApiResult.Success(period.copy(endedAt = "2026-09-08T13:00:00Z"))); runCurrent()
        assertFalse(controller.state.value.sources.single().paused)
    }

    @Test fun `resuming an old period preserves a newer authoritative pause`() = runTest {
        val gateway = FakeGateway(paused = true)
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        controller.resume(controller.state.value.sources.single()); runCurrent()
        gateway.result.complete(ApiResult.Success(period.copy(endedAt = "2026-09-08T13:00:00Z"))); runCurrent()
        assertTrue(controller.state.value.sources.single().paused)
    }

    @Test fun `a refresh started before pause cannot overwrite confirmation`() = runTest {
        val gateway = FakeGateway()
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        val staleSources = CompletableDeferred<ApiResult<List<PauseSource>>>()
        gateway.delayedSources = staleSources
        controller.refresh(); runCurrent()
        controller.edit(source); controller.reason(PauseReason.OTHER); controller.submit(); runCurrent()
        gateway.delayedSources = null
        gateway.paused = true
        gateway.result.complete(ApiResult.Success(period)); runCurrent()
        assertTrue(controller.state.value.sources.single().paused)
        staleSources.complete(ApiResult.Success(listOf(source))); runCurrent()
        assertTrue(controller.state.value.sources.single().paused)
    }

    @Test fun `a pending resume prevents another treatment pause from opening`() = runTest {
        val gateway = FakeGateway(paused = true)
        gateway.extraSources = listOf(source.copy(id = "other"))
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        controller.resume(controller.state.value.sources.first()); runCurrent()
        controller.edit(controller.state.value.sources.last())
        assertNull(controller.state.value.editing)
        assertEquals(0, gateway.requests)
    }

    @Test fun `replayed accepted pause does not hide a subsequently resumed treatment`() = runTest {
        val gateway = FakeGateway()
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        controller.edit(source); controller.reason(PauseReason.OTHER); controller.submit(); runCurrent()
        gateway.result.complete(ApiResult.Success(period)); runCurrent()
        assertFalse(controller.state.value.sources.single().paused)
    }

    @Test fun `inactive paused assignment resumes and refreshes authoritative state`() = runTest {
        val gateway = FakeGateway(paused = true, sourceType = "person_medication")
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        val paused = controller.state.value.pausedSources(null).single()
        assertFalse(paused.active)
        controller.resume(paused); runCurrent()
        assertEquals(paused.key, controller.state.value.busySource)
        gateway.paused = false
        gateway.result.complete(ApiResult.Success(period.copy(endedAt = "2026-09-08T13:00:00Z"))); runCurrent()
        assertFalse(controller.state.value.sources.single().paused)
        assertTrue(controller.state.value.sources.single().active)
    }

    @Test fun `read only sources keep history but reject pause and resume commands`() = runTest {
        val gateway = FakeGateway(canManage = false)
        val controller = MedicationPauseController(AppSession("https://example.test", null), gateway, backgroundScope, { true }, {})
        controller.refresh(); runCurrent()
        val active = controller.state.value.sources.single()

        controller.edit(active)
        assertNull(controller.state.value.editing)

        gateway.paused = true
        controller.refresh(); runCurrent()
        val paused = controller.state.value.sources.single()
        controller.resume(paused); runCurrent()
        assertNull(controller.state.value.busySource)

        controller.showHistory(paused); runCurrent()
        assertEquals(paused, controller.state.value.historySource)
        assertEquals(listOf(period), controller.state.value.history)
        assertEquals(0, gateway.requests)
    }

    private inner class FakeGateway(supported: Boolean = true, var paused: Boolean = false, val sourceType: String = "schedule", val canManage: Boolean = true) : MedicationPauseGateway {
        val result = CompletableDeferred<ApiResult<PausePeriod>>()
        var capability: ApiResult<Boolean> = ApiResult.Success(supported)
        var requests = 0
        var sourceRequests = 0
        val requestIds = mutableListOf<String>()
        var extraSources = emptyList<PauseSource>()
        var delayedSources: CompletableDeferred<ApiResult<List<PauseSource>>>? = null
        override suspend fun supported(session: AppSession) = capability
        override suspend fun sources(session: AppSession): ApiResult<List<PauseSource>> {
            sourceRequests++
            delayedSources?.let { return it.await() }
            return ApiResult.Success(listOf(source.copy(type = sourceType, active = !paused, paused = paused, currentPauseId = if (paused) "period" else null, canManage = canManage)) + extraSources)
        }
        override suspend fun history(session: AppSession, source: PauseSource) = ApiResult.Success(listOf(period))
        override suspend fun pause(session: AppSession, source: PauseSource, reason: PauseReason, note: String, requestId: String): ApiResult<PausePeriod> {
            requests++
            requestIds.add(requestId)
            return result.await()
        }
        override suspend fun resume(session: AppSession, periodId: String) = result.await()
    }
}
