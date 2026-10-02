package io.damacus.medtracker.ui.dashboard

import io.damacus.medtracker.data.model.*
import org.junit.Assert.*
import org.junit.Test

class MedicationPauseTest {
    @Test fun `all five reasons require an explicit selection and Other allows no note`() {
        assertEquals(listOf("out_of_supply", "temporarily_not_needed", "clinician_advice", "side_effects", "other"), PauseReason.entries.map { it.value })
        assertFalse(PauseForm().canSubmit)
        assertTrue(PauseForm(reason = PauseReason.OTHER).canSubmit)
        assertFalse(PauseForm(reason = PauseReason.OTHER, submitting = true).canSubmit)
    }

    @Test fun `paused discovery includes assignments and schedules outside today and respects person selection`() {
        val sources = listOf(
            PauseSource("schedule", "schedule-id", 1, "Weekly treatment", true),
            PauseSource("person_medication", "assignment-id", 2, "PRN treatment", true),
            PauseSource("schedule", "active-id", 1, "Active treatment", false)
        )
        val state = MedicationPauseState(sources = sources)
        assertEquals(2, state.pausedSources(null).size)
        assertEquals(listOf("assignment-id"), state.pausedSources(2).map { it.id })
    }

    @Test fun `history sorts newest first and preserves unknown legacy context`() {
        val old = PausePeriod("old", "schedule", "source", null, null, null, null, true, null, null)
        val recent = old.copy(id = "new", reason = "other", startedAt = "2026-09-08T09:00:00Z", note = "Optional detail", createdAt = "2026-09-08T09:00:00Z")
        assertEquals(listOf("new", "old"), MedicationPauseState(history = listOf(old, recent)).orderedHistory.map { it.id })
        assertEquals("Reason not recorded", old.reasonLabel)
        assertEquals("Start date not recorded", old.startLabel)
        assertEquals("Not recorded", old.recordedByLabel)
    }

    @Test fun `recent legacy period with unknown start sorts by recording date`() {
        val legacy = PausePeriod("legacy", "schedule", "source", null, null, null, null, true, null, null, "2026-09-08T12:00:00Z")
        val earlier = legacy.copy(id = "earlier", startedAt = "2026-09-07T12:00:00Z", createdAt = "2026-09-07T12:00:00Z")
        assertEquals("legacy", MedicationPauseState(history = listOf(earlier, legacy)).orderedHistory.first().id)
        assertFalse(formatPauseDate("2026-09-08T12:00:00Z").contains("T12:00:00Z"))
    }

    @Test fun `active schedules beyond dashboard page remain discoverable without duplicating existing controls`() {
        val sources = (1..25).map { PauseSource("schedule", "schedule-$it", 1, "Medicine $it", false) }
        val visible = (1..20).map { "schedule-$it" }.toSet()
        assertEquals(5, MedicationPauseState(sources = sources).additionalActiveSources(1, visible).size)
        assertTrue(MedicationPauseState(sources = sources).additionalActiveSources(2, visible).isEmpty())
    }
}
