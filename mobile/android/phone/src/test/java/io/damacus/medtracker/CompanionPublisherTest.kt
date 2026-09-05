package io.damacus.medtracker

import io.damacus.medtracker.companion.CompanionPublisher
import io.damacus.medtracker.companion.CompanionTransport
import io.damacus.medtracker.companion.PhoneDataLayer
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.common.api.Status
import com.google.android.gms.wearable.WearableStatusCodes
import io.damacus.medtracker.wear.protocol.*
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.*
import org.junit.Test

class CompanionPublisherTest {
    @Test fun `already advertised capability does not block persistent publication`() = runBlocking {
        val publications = mutableListOf<ByteArray>()
        val transport = PhoneDataLayer(
            announce = { throw ApiException(Status(WearableStatusCodes.DUPLICATE_CAPABILITY)) },
            put = { _, bytes -> publications.add(bytes) }
        )
        withTimeout(1_000) { CompanionPublisher(transport, "1").observe(flowOf(false)) }
        assertEquals(1, publications.size)
    }

    @Test fun `retries unavailable publication using current session rather than stale signed in state`() = runBlocking {
        val session = MutableStateFlow(true)
        val published = CompletableDeferred<ByteArray>()
        var attempts = 0
        val transport = object : CompanionTransport {
            override suspend fun advertise(capability: String) = Unit
            override suspend fun publish(path: String, data: ByteArray) {
                attempts += 1
                if (attempts == 1) {
                    session.value = false
                    throw IllegalStateException("Data Layer unavailable")
                }
                published.complete(data)
            }
        }
        val observation = launch { CompanionPublisher(transport, "1.2", { 42L }).observe(session) }
        try {
            val result = CompanionProtocol.decode(withTimeout(7_000) { published.await() }) as DecodedStatus.Supported
            assertEquals(SessionState.SIGNED_OUT, result.status.sessionState)
            assertEquals(2, attempts)
        } finally {
            observation.cancel()
        }
    }

    @Test fun `advertises and persists initial sign in and sign out without duplicate states`() = runBlocking {
        val capabilities = mutableListOf<String>()
        val publications = mutableListOf<Pair<String, ByteArray>>()
        val transport = object : CompanionTransport {
            override suspend fun advertise(capability: String) { capabilities.add(capability) }
            override suspend fun publish(path: String, data: ByteArray) { publications.add(path to data) }
        }
        CompanionPublisher(transport, "1.2", { 42L }).observe(flowOf(false, false, true, false))
        assertEquals(listOf(CompanionProtocol.CAPABILITY), capabilities)
        assertEquals(3, publications.size)
        assertTrue(publications.all { it.first == CompanionProtocol.STATUS_PATH })
        assertEquals(listOf(SessionState.SIGNED_OUT, SessionState.SIGNED_IN, SessionState.SIGNED_OUT), publications.map {
            val decoded = CompanionProtocol.decode(it.second) as DecodedStatus.Supported
            assertEquals("1.2", decoded.status.phoneAppVersion)
            assertEquals(42L, decoded.status.publishedAt)
            decoded.status.sessionState
        })
    }
}
