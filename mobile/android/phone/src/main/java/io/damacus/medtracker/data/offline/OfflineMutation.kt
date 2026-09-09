package io.damacus.medtracker.data.offline

import java.util.UUID

enum class MutationType {
    RECORD_DOSE,
    RECORD_NOT_TAKEN,
    RECORD_STOCK_REMOVAL,
    UPDATE_MEDICATION,
    CREATE_SCHEDULE,
    PAUSE_SCHEDULE,
    RESUME_SCHEDULE
}

data class OfflineMutation(
    val id: String = UUID.randomUUID().toString(),
    val householdId: Long,
    val type: MutationType,
    val payloadJson: String,
    val createdAtMs: Long = System.currentTimeMillis(),
    val attempts: Int = 0,
    val lastError: String? = null
)
