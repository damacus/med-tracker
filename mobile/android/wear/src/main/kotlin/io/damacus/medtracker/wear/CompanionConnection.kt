package io.damacus.medtracker.wear

import io.damacus.medtracker.wear.protocol.CompanionProtocol
import io.damacus.medtracker.wear.protocol.DecodedStatus
import io.damacus.medtracker.wear.protocol.SessionState

enum class ConnectionState {
    PHONE_APP_MISSING, DISCONNECTED, WAITING_FOR_STATUS, INCOMPATIBLE, SIGNED_OUT, READY
}

class CompanionConnection {
    private var known = emptySet<String>()
    private var reachable = emptySet<String>()
    private var discoveryAvailable = false
    private val statuses = mutableMapOf<String, DecodedStatus>()

    val state: ConnectionState
        get() {
            if (!discoveryAvailable) return ConnectionState.DISCONNECTED
            if (known.isEmpty()) return ConnectionState.PHONE_APP_MISSING
            val node = reachable.intersect(known).sorted().firstOrNull() ?: return ConnectionState.DISCONNECTED
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
}
