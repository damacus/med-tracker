package io.damacus.medtracker.wear

import android.content.Context
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.wearable.CapabilityClient
import com.google.android.gms.wearable.DataClient
import com.google.android.gms.wearable.PutDataRequest
import com.google.android.gms.wearable.Wearable
import com.google.android.gms.wearable.WearableStatusCodes
import io.damacus.medtracker.wear.protocol.CompanionProtocol
import io.damacus.medtracker.wear.protocol.CompanionSchedule
import io.damacus.medtracker.wear.protocol.DoseProtocol
import io.damacus.medtracker.wear.protocol.ScheduleProtocol
import io.damacus.medtracker.wear.protocol.TakenDoseRecord
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

data class CompanionSnapshot(
    val state: ConnectionState,
    val schedule: CompanionSchedule?,
    val isPhoneReachable: Boolean
)

class WearDataLayer(private val context: Context) {
    private val capabilityClient = Wearable.getCapabilityClient(context)
    private val dataClient = Wearable.getDataClient(context)
    private val updates = Channel<Unit>(Channel.CONFLATED)
    private val capabilityListener = CapabilityClient.OnCapabilityChangedListener { updates.trySend(Unit) }
    private val dataListener = DataClient.OnDataChangedListener { events ->
        if (events.any { it.dataItem.uri.path == CompanionProtocol.STATUS_PATH || it.dataItem.uri.path == ScheduleProtocol.PATH }) {
            updates.trySend(Unit)
        }
    }

    suspend fun recordDose(record: TakenDoseRecord) {
        val path = "${DoseProtocol.PATH_PREFIX}${record.takenAt}_${record.eventId}"
        val request = PutDataRequest.create(path)
            .setData(DoseProtocol.encode(record))
            .setUrgent()
        dataClient.putDataItem(request).await()
    }

    suspend fun observe(onState: (ConnectionState) -> Unit) {
        observeSnapshot { onState(it.state) }
    }

    suspend fun observeSnapshot(onSnapshot: (CompanionSnapshot) -> Unit) {
        val connection = CompanionConnection()
        var capabilityListening = false
        var dataListening = false
        try {
            while (currentCoroutineContext().isActive) {
                var reachableNodes = emptySet<String>()
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
                    reachableNodes = reachable
                    val buffer = dataClient.dataItems.await()
                    val statusItems = mutableMapOf<String, ByteArray>()
                    val scheduleItems = mutableMapOf<String, ByteArray>()
                    try {
                        buffer.forEach { item ->
                            val node = item.uri.host
                            val bytes = item.data
                            if (node != null && bytes != null) {
                                if (item.uri.path == CompanionProtocol.STATUS_PATH) {
                                    statusItems[node] = bytes.copyOf()
                                } else if (item.uri.path == ScheduleProtocol.PATH) {
                                    scheduleItems[node] = bytes.copyOf()
                                }
                            }
                        }
                    } finally {
                        buffer.release()
                    }
                    connection.replaceStatuses(statusItems)
                    connection.replaceSchedules(scheduleItems)
                    connection.discovery(known, reachable)
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (_: Exception) {
                    connection.discoveryFailed()
                }
                val active = connection.activeNode
                val isReachable = active != null && reachableNodes.contains(active)
                onSnapshot(CompanionSnapshot(connection.state, connection.currentSchedule, isReachable))
                withTimeoutOrNull(5_000) { updates.receive() }
            }
        } finally {
            capabilityClient.removeListener(capabilityListener, CompanionProtocol.CAPABILITY)
            dataClient.removeListener(dataListener)
            updates.close()
        }
    }
}
