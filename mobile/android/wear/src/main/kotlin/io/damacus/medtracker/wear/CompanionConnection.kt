package io.damacus.medtracker.wear

import io.damacus.medtracker.wear.protocol.CompanionProtocol
import io.damacus.medtracker.wear.protocol.CompanionSchedule
import io.damacus.medtracker.wear.protocol.DecodedStatus
import io.damacus.medtracker.wear.protocol.ScheduleProtocol
import io.damacus.medtracker.wear.protocol.SessionState

enum class ConnectionState {
    PHONE_APP_MISSING, DISCONNECTED, WAITING_FOR_STATUS, INCOMPATIBLE, SIGNED_OUT, READY
}

class CompanionConnection {
    private var known = emptySet<String>()
    private var reachable = emptySet<String>()
    private var discoveryAvailable = false
    private val statuses = mutableMapOf<String, DecodedStatus>()
    private val schedules = mutableMapOf<String, CompanionSchedule>()

    val activeNode: String?
        get() = reachable.intersect(known).sorted().firstOrNull()

    val currentSchedule: CompanionSchedule?
        get() = activeNode?.let { schedules[it] }

    val state: ConnectionState
        get() {
            if (!discoveryAvailable) return ConnectionState.DISCONNECTED
            if (known.isEmpty()) return ConnectionState.PHONE_APP_MISSING
            val node = activeNode ?: return ConnectionState.DISCONNECTED
            return when (val status = statuses[node]) {
                null -> ConnectionState.WAITING_FOR_STATUS
                is DecodedStatus.Supported -> if (status.status.sessionState == SessionState.SIGNED_IN) {
                    ConnectionState.READY
                } else {
                    ConnectionState.SIGNED_OUT
                }
                else -> ConnectionState.INCOMPATIBLE
            }
        }

    fun discovery(known: Set<String>, reachable: Set<String>) {
        this.known = known
        this.reachable = reachable
        discoveryAvailable = true
    }

    fun discoveryFailed() {
        discoveryAvailable = false
    }

    fun status(node: String, data: ByteArray?) {
        if (data == null) statuses.remove(node) else statuses[node] = CompanionProtocol.decode(data)
    }

    fun replaceStatuses(items: Map<String, ByteArray>) {
        statuses.clear()
        items.forEach { (node, data) -> status(node, data) }
    }

    fun schedule(node: String, data: ByteArray?) {
        if (data == null) schedules.remove(node) else {
            val decoded = ScheduleProtocol.decode(data)
            if (decoded != null) schedules[node] = decoded
        }
    }

    fun replaceSchedules(items: Map<String, ByteArray>) {
        schedules.clear()
        items.forEach { (node, data) -> schedule(node, data) }
    }
}
