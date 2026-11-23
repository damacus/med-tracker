package io.damacus.medtracker.data.offline

import android.content.Context
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption

interface OfflineQueueStore {
    fun load(): List<OfflineMutation>
    fun save(mutations: List<OfflineMutation>)
}

class InMemoryOfflineQueueStore(
    initial: List<OfflineMutation> = emptyList()
) : OfflineQueueStore {
    private var snapshot: List<OfflineMutation> = initial

    override fun load(): List<OfflineMutation> = snapshot

    override fun save(mutations: List<OfflineMutation>) {
        snapshot = mutations
    }
}

class FileOfflineQueueStore(
    private val file: File,
    private val json: Json = defaultJson
) : OfflineQueueStore {

    override fun load(): List<OfflineMutation> {
        if (!file.exists()) return emptyList()
        return runCatching {
            val content = file.readText()
            if (content.isBlank()) {
                emptyList()
            } else {
                json.decodeFromString<List<OfflineMutation>>(content)
            }
        }.getOrElse { emptyList() }
    }

    override fun save(mutations: List<OfflineMutation>) {
        val parent = file.parentFile
        if (parent != null && !parent.exists()) {
            parent.mkdirs()
        }
        val tmpFile = File(parent ?: file.absoluteFile.parentFile, "${file.name}.tmp")
        tmpFile.writeText(json.encodeToString(mutations))
        try {
            Files.move(
                tmpFile.toPath(),
                file.toPath(),
                StandardCopyOption.REPLACE_EXISTING,
                StandardCopyOption.ATOMIC_MOVE
            )
        } catch (_: AtomicMoveNotSupportedException) {
            Files.move(
                tmpFile.toPath(),
                file.toPath(),
                StandardCopyOption.REPLACE_EXISTING
            )
        }
    }

    companion object {
        private val defaultJson = Json {
            ignoreUnknownKeys = true
            encodeDefaults = true
        }
    }
}

data class FlushResult(
    val succeeded: Int,
    val failed: Int,
    val remaining: Int
)

class OfflineQueueRepository(
    private val store: OfflineQueueStore = InMemoryOfflineQueueStore()
) {
    private val mutex = Mutex()
    private val _pendingMutations = MutableStateFlow(store.load())
    val pendingMutations: StateFlow<List<OfflineMutation>> = _pendingMutations.asStateFlow()

    suspend fun enqueue(mutation: OfflineMutation) {
        mutex.withLock {
            val updated = _pendingMutations.value + mutation
            store.save(updated)
            _pendingMutations.value = updated
        }
    }

    suspend fun dequeue(mutationId: String) {
        mutex.withLock {
            val updated = _pendingMutations.value.filterNot { it.id == mutationId }
            store.save(updated)
            _pendingMutations.value = updated
        }
    }

    suspend fun markFailed(mutationId: String, error: String) {
        mutex.withLock {
            val updated = _pendingMutations.value.map {
                if (it.id == mutationId) {
                    it.copy(attempts = it.attempts + 1, lastError = error)
                } else {
                    it
                }
            }
            store.save(updated)
            _pendingMutations.value = updated
        }
    }

    suspend fun clear() {
        mutex.withLock {
            store.save(emptyList())
            _pendingMutations.value = emptyList()
        }
    }

    suspend fun flushPending(
        executor: suspend (OfflineMutation) -> Result<Unit>
    ): FlushResult {
        val snapshot = mutex.withLock { _pendingMutations.value }
        var succeeded = 0
        var failed = 0
        for (mutation in snapshot) {
            val outcome = try {
                executor(mutation)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                Result.failure(error)
            }
            if (outcome.isSuccess) {
                dequeue(mutation.id)
                succeeded++
            } else {
                markFailed(
                    mutation.id,
                    outcome.exceptionOrNull()?.message ?: "Replay failed"
                )
                failed++
            }
        }
        return FlushResult(
            succeeded = succeeded,
            failed = failed,
            remaining = pendingCount()
        )
    }

    fun pendingCount(): Int = _pendingMutations.value.size

    companion object {
        const val QUEUE_FILE_NAME = "offline_queue.json"

        fun createPersistent(context: Context): OfflineQueueRepository {
            val file = File(context.noBackupFilesDir, QUEUE_FILE_NAME)
            return OfflineQueueRepository(FileOfflineQueueStore(file))
        }
    }
}
