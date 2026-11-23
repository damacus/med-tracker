package io.damacus.medtracker.data.offline

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File

class OfflineQueueRepositoryTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    private lateinit var repository: OfflineQueueRepository

    @Before
    fun setUp() {
        repository = OfflineQueueRepository()
    }

    @Test
    fun enqueueAddsMutationToPendingList() = runBlocking {
        assertEquals(0, repository.pendingCount())

        val mutation = OfflineMutation(
            householdId = 100L,
            type = MutationType.RECORD_DOSE,
            payloadJson = "{}"
        )
        repository.enqueue(mutation)

        assertEquals(1, repository.pendingCount())
        assertEquals(mutation, repository.pendingMutations.value.first())
    }

    @Test
    fun dequeueRemovesMutationFromPendingList() = runBlocking {
        val mutation = OfflineMutation(
            householdId = 100L,
            type = MutationType.RECORD_STOCK_REMOVAL,
            payloadJson = "{}"
        )
        repository.enqueue(mutation)
        assertEquals(1, repository.pendingCount())

        repository.dequeue(mutation.id)
        assertEquals(0, repository.pendingCount())
    }

    @Test
    fun markFailedIncrementsAttemptsAndSetsError() = runBlocking {
        val mutation = OfflineMutation(
            householdId = 100L,
            type = MutationType.UPDATE_MEDICATION,
            payloadJson = "{}"
        )
        repository.enqueue(mutation)

        repository.markFailed(mutation.id, "500 Server Error")
        val updated = repository.pendingMutations.value.first()

        assertEquals(1, updated.attempts)
        assertEquals("500 Server Error", updated.lastError)
    }

    @Test
    fun clearEmptiesPendingList() = runBlocking {
        repository.enqueue(OfflineMutation(householdId = 1L, type = MutationType.RECORD_DOSE, payloadJson = "{}"))
        repository.enqueue(OfflineMutation(householdId = 2L, type = MutationType.CREATE_SCHEDULE, payloadJson = "{}"))
        assertEquals(2, repository.pendingCount())

        repository.clear()
        assertTrue(repository.pendingMutations.value.isEmpty())
    }

    @Test
    fun fileStorePersistsMutationsAcrossRepositoryRestarts() = runBlocking {
        val queueFile = File(tempFolder.root, OfflineQueueRepository.QUEUE_FILE_NAME)
        val firstRepo = OfflineQueueRepository(FileOfflineQueueStore(queueFile))

        val doseMutation = OfflineMutation(
            id = "mut-1",
            householdId = 42L,
            type = MutationType.RECORD_DOSE,
            payloadJson = """{"scheduleId":9}""",
            createdAtMs = 1_700_000_000_000L
        )
        firstRepo.enqueue(doseMutation)
        firstRepo.markFailed("mut-1", "timeout")

        assertTrue(queueFile.exists())

        val reloadedRepo = OfflineQueueRepository(FileOfflineQueueStore(queueFile))
        assertEquals(1, reloadedRepo.pendingCount())
        val restored = reloadedRepo.pendingMutations.value.single()
        assertEquals("mut-1", restored.id)
        assertEquals(42L, restored.householdId)
        assertEquals(MutationType.RECORD_DOSE, restored.type)
        assertEquals("""{"scheduleId":9}""", restored.payloadJson)
        assertEquals(1, restored.attempts)
        assertEquals("timeout", restored.lastError)
    }

    @Test
    fun fileStoreRecoversGracefullyFromCorruptJson() {
        val queueFile = File(tempFolder.root, OfflineQueueRepository.QUEUE_FILE_NAME)
        queueFile.writeText("{not-valid-json")

        val repo = OfflineQueueRepository(FileOfflineQueueStore(queueFile))
        assertEquals(0, repo.pendingCount())
    }

    @Test
    fun flushPendingReplaysInOrderDequeuesSuccessesAndMarksFailures() = runBlocking {
        val queueFile = File(tempFolder.root, OfflineQueueRepository.QUEUE_FILE_NAME)
        val repo = OfflineQueueRepository(FileOfflineQueueStore(queueFile))

        val first = OfflineMutation(id = "m1", householdId = 1L, type = MutationType.RECORD_DOSE, payloadJson = "{}")
        val second = OfflineMutation(id = "m2", householdId = 1L, type = MutationType.RECORD_NOT_TAKEN, payloadJson = "{}")
        val third = OfflineMutation(id = "m3", householdId = 1L, type = MutationType.PAUSE_SCHEDULE, payloadJson = "{}")
        repo.enqueue(first)
        repo.enqueue(second)
        repo.enqueue(third)

        val executedIds = mutableListOf<String>()
        val result = repo.flushPending { mutation ->
            executedIds.add(mutation.id)
            if (mutation.id == "m2") {
                Result.failure(IllegalStateException("Network unavailable"))
            } else {
                Result.success(Unit)
            }
        }

        assertEquals(listOf("m1", "m2", "m3"), executedIds)
        assertEquals(2, result.succeeded)
        assertEquals(1, result.failed)
        assertEquals(1, result.remaining)

        val remaining = repo.pendingMutations.value.single()
        assertEquals("m2", remaining.id)
        assertEquals(1, remaining.attempts)
        assertEquals("Network unavailable", remaining.lastError)

        // Verify persisted state matches flushed state
        val reloaded = OfflineQueueRepository(FileOfflineQueueStore(queueFile))
        assertEquals(listOf("m2"), reloaded.pendingMutations.value.map { it.id })
    }
}
