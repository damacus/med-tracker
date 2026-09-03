package io.damacus.medtracker.wear

import android.content.Context
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.wearable.CapabilityClient
import com.google.android.gms.wearable.DataClient
import com.google.android.gms.wearable.Wearable
import com.google.android.gms.wearable.WearableStatusCodes
import io.damacus.medtracker.wear.protocol.CompanionProtocol
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.isActive
import kotlinx.coroutines.tasks.await
import kotlinx.coroutines.withTimeoutOrNull

internal suspend fun discoverCapabilityNodes(query: suspend () -> Set<String>): Set<String> = try {
    query()
} catch (error: ApiException) {
    if (error.statusCode == WearableStatusCodes.UNKNOWN_CAPABILITY) emptySet() else throw error
}

class WearDataLayer(private val context: Context) {
    private val capabilityClient = Wearable.getCapabilityClient(context)
    private val dataClient = Wearable.getDataClient(context)
    private val updates = Channel<Unit>(Channel.CONFLATED)
    private val capabilityListener = CapabilityClient.OnCapabilityChangedListener { updates.trySend(Unit) }
    private val dataListener = DataClient.OnDataChangedListener { events ->
        if (events.any { it.dataItem.uri.path == CompanionProtocol.STATUS_PATH }) updates.trySend(Unit)
    }

    suspend fun observe(onState: (ConnectionState) -> Unit) {
        val connection = CompanionConnection()
        var capabilityListening = false
        var dataListening = false
        try {
            while (currentCoroutineContext().isActive) {
                try {
                    check(GoogleApiAvailability.getInstance().isGooglePlayServicesAvailable(context) == ConnectionResult.SUCCESS)
                    if (!capabilityListening) {
                        capabilityClient.addListener(capabilityListener, CompanionProtocol.CAPABILITY).await()
                        capabilityListening = true
                    }
                    if (!dataListening) {
                        dataClient.addListener(dataListener).await()
                        dataListening = true
                    }
                    val known = discoverCapabilityNodes {
                        capabilityClient.getCapability(CompanionProtocol.CAPABILITY, CapabilityClient.FILTER_ALL).await().nodes.map { it.id }.toSet()
                    }
                    val reachable = discoverCapabilityNodes {
                        capabilityClient.getCapability(CompanionProtocol.CAPABILITY, CapabilityClient.FILTER_REACHABLE).await().nodes.map { it.id }.toSet()
                    }
                    val buffer = dataClient.dataItems.await()
                    val items = try {
                        buildMap {
                            buffer.forEach { item ->
                                val node = item.uri.host
                                val bytes = item.data
                                if (item.uri.path == CompanionProtocol.STATUS_PATH && node != null && bytes != null) {
                                    put(node, bytes.copyOf())
                                }
                            }
                        }
                    } finally {
                        buffer.release()
                    }
                    connection.replaceStatuses(items)
                    connection.discovery(known, reachable)
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (_: Exception) {
                    connection.discoveryFailed()
                }
                onState(connection.state)
                withTimeoutOrNull(5_000) { updates.receive() }
            }
        } finally {
            capabilityClient.removeListener(capabilityListener, CompanionProtocol.CAPABILITY)
            dataClient.removeListener(dataListener)
            updates.close()
        }
    }
}
