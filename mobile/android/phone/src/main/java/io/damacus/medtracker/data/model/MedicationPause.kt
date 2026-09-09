package io.damacus.medtracker.data.model

import java.time.OffsetDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import java.util.UUID

enum class PauseReason(val value: String, val label: String) {
    OUT_OF_SUPPLY("out_of_supply", "Out of supply"),
    TEMPORARILY_NOT_NEEDED("temporarily_not_needed", "Temporarily not needed"),
    CLINICIAN_ADVICE("clinician_advice", "Clinician advice"),
    SIDE_EFFECTS("side_effects", "Side effects"),
    OTHER("other", "Other")
}

data class PauseForm(
    val reason: PauseReason? = null,
    val note: String = "",
    val submitting: Boolean = false,
    val error: String? = null,
    val requestId: String = UUID.randomUUID().toString()
) {
    val canSubmit get() = reason != null && !submitting
}

data class PauseSource(
    val type: String,
    val id: String,
    val personId: Long,
    val name: String,
    val paused: Boolean,
    val currentPauseId: String? = null,
    val personName: String = "",
    val active: Boolean = true,
    val currentReason: String? = null,
    val currentNote: String? = null,
    val canManage: Boolean = true
) {
    val key get() = "$type:$id"
}

data class PausePeriod(
    val id: String,
    val sourceType: String,
    val sourceId: String,
    val reason: String?,
    val note: String?,
    val startedAt: String?,
    val endedAt: String?,
    val legacyContext: Boolean,
    val recordedBy: String?,
    val resumedBy: String?,
    val createdAt: String? = startedAt
) {
    val reasonLabel get() = PauseReason.entries.find { it.value == reason }?.label ?: "Reason not recorded"
    val startLabel get() = startedAt?.let(::formatPauseDate) ?: "Start date not recorded"
    val recordedByLabel get() = recordedBy ?: "Not recorded"
}

data class MedicationPauseState(
    val supported: Boolean = false,
    val sources: List<PauseSource> = emptyList(),
    val editing: PauseSource? = null,
    val historySource: PauseSource? = null,
    val history: List<PausePeriod> = emptyList(),
    val form: PauseForm = PauseForm(),
    val loadingHistory: Boolean = false,
    val busySource: String? = null,
    val error: String? = null
) {
    fun pausedSources(personId: Long?) = sources.filter { it.paused && (personId == null || it.personId == personId) }
    fun activeSources(personId: Long?) = sources.filter { it.active && !it.paused && (personId == null || it.personId == personId) }
    fun additionalActiveSources(personId: Long?, displayedScheduleIds: Set<String>) = activeSources(personId).filter { it.type != "schedule" || it.id !in displayedScheduleIds }
    val orderedHistory get() = history.sortedWith(compareByDescending<PausePeriod> { it.createdAt?.let { date -> OffsetDateTime.parse(date).toInstant() } }.thenByDescending { it.id })
}

fun formatPauseDate(value: String): String = OffsetDateTime.parse(value).atZoneSameInstant(ZoneId.systemDefault()).format(DateTimeFormatter.ofLocalizedDateTime(FormatStyle.MEDIUM))
