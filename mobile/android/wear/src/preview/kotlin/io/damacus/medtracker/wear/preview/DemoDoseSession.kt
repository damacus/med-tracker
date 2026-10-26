package io.damacus.medtracker.wear.preview

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

data class DemoDose(val id: Int, val name: String, val amount: String, val time: String)

class DemoDoseSession(initialRecorded: Set<Int> = emptySet()) {
    val doses = listOf(
        DemoDose(1, "Vitamin D", "1 tablet", "09:00"),
        DemoDose(2, "Vitamin B12", "1 tablet", "12:00"),
        DemoDose(3, "Vitamin C", "1 tablet", "18:00")
    )
    private val recordedState = MutableStateFlow(initialRecorded.intersect(doses.map { it.id }.toSet()))
    val recorded = recordedState.asStateFlow()

    fun record(id: Int): Boolean {
        if (doses.none { it.id == id }) return false
        while (true) {
            val previous = recordedState.value
            if (id in previous) return false
            if (recordedState.compareAndSet(previous, previous + id)) return true
        }
    }
}
