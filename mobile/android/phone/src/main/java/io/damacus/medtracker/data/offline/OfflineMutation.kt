package io.damacus.medtracker.data.offline

import kotlinx.serialization.Serializable
import io.damacus.medtracker.data.SessionOwner
import java.util.UUID

@Serializable
enum class MutationType {
    RECORD_DOSE,
    RECORD_NOT_TAKEN,
    RECORD_STOCK_REMOVAL,
    UPDATE_MEDICATION,
    CREATE_SCHEDULE,
    PAUSE_SCHEDULE,
    RESUME_SCHEDULE
}

@Serializable
data class OfflineMutation(
    val id: String = UUID.randomUUID().toString(),
    val householdId: Long,
    val type: MutationType,
    val payloadJson: String,
    val createdAtMs: Long = System.currentTimeMillis(),
    val attempts: Int = 0,
    val lastError: String? = null,
    val owner: SessionOwner? = null
)
