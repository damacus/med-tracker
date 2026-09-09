package io.damacus.medtracker.data.offline

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class OfflineQueueRepositoryTest {

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
}
