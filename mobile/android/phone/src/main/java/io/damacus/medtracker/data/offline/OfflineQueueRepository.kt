package io.damacus.medtracker.data.offline

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class OfflineQueueRepository {
    private val mutex = Mutex()
    private val _pendingMutations = MutableStateFlow<List<OfflineMutation>>(emptyList())
    val pendingMutations: StateFlow<List<OfflineMutation>> = _pendingMutations.asStateFlow()

    suspend fun enqueue(mutation: OfflineMutation) {
        mutex.withLock {
            _pendingMutations.value = _pendingMutations.value + mutation
        }
    }

    suspend fun dequeue(mutationId: String) {
        mutex.withLock {
            _pendingMutations.value = _pendingMutations.value.filterNot { it.id == mutationId }
        }
    }

    suspend fun markFailed(mutationId: String, error: String) {
        mutex.withLock {
            _pendingMutations.value = _pendingMutations.value.map {
                if (it.id == mutationId) {
                    it.copy(attempts = it.attempts + 1, lastError = error)
                } else {
                    it
                }
            }
        }
    }

    suspend fun clear() {
        mutex.withLock {
            _pendingMutations.value = emptyList()
        }
    }

    fun pendingCount(): Int = _pendingMutations.value.size
}
