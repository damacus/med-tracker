package io.damacus.medtracker.companion

import io.damacus.medtracker.wear.protocol.CompanionProtocol
import io.damacus.medtracker.wear.protocol.CompanionStatus
import io.damacus.medtracker.wear.protocol.SessionState
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.emitAll
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.flow.retryWhen

interface CompanionTransport {
    suspend fun advertise(capability: String)
    suspend fun publish(path: String, data: ByteArray)
}

class CompanionPublisher(
    private val transport: CompanionTransport,
    private val phoneAppVersion: String,
    private val now: () -> Long = System::currentTimeMillis
) {
    suspend fun observe(signedIn: Flow<Boolean>) {
        flow {
            transport.advertise(CompanionProtocol.CAPABILITY)
            emitAll(signedIn.distinctUntilChanged())
        }.onEach { authenticated ->
            transport.publish(CompanionProtocol.STATUS_PATH, CompanionProtocol.encode(CompanionStatus(
                phoneAppVersion,
                if (authenticated) SessionState.SIGNED_IN else SessionState.SIGNED_OUT,
                now()
            )))
        }.retryWhen { _, _ ->
            delay(5_000)
            true
        }.collect()
    }
}
