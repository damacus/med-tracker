package io.damacus.medtracker.notifications

import io.damacus.medtracker.data.api.DoseOccurrenceDto
import io.damacus.medtracker.data.SessionOwner
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.ScheduleDto
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.time.Clock
import java.time.Instant
import java.time.LocalTime
import java.time.ZoneOffset
import java.time.ZonedDateTime

class ReminderSchedulerTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    private class FakeReminderAlarmBackend : ReminderAlarmBackend {
        val scheduled = mutableListOf<ScheduledReminder>()
        val cancelledIds = mutableListOf<Long>()

        override fun schedule(reminder: ScheduledReminder) {
            scheduled.removeAll { it.scheduleId == reminder.scheduleId }
            scheduled.add(reminder)
        }

        override fun cancel(requestCode: Int, scheduleId: Long) {
            scheduled.removeAll { it.scheduleId == scheduleId }
            cancelledIds.add(scheduleId)
        }
    }

    @Test
    fun syncSchedulesSchedulesActiveNonPausedSchedulesOnly() {
        val backend = FakeReminderAlarmBackend()
        val fixedNow = ZonedDateTime.of(2026, 6, 15, 7, 0, 0, 0, ZoneOffset.UTC)
        val scheduler = ReminderScheduler(backend, Clock.fixed(fixedNow.toInstant(), ZoneOffset.UTC))

        val schedules = listOf(
            ScheduleDto(
                id = 1L,
                personId = 10L,
                medicationId = 100L,
                doseAmount = 500.0,
                doseUnit = "mg",
                doseCycle = "morning",
                active = true,
                paused = false
            ),
            ScheduleDto(
                id = 2L,
                personId = 10L,
                medicationId = 101L,
                doseAmount = 1.0,
                doseUnit = "tablet",
                doseCycle = "evening",
                active = true,
                paused = true
            ),
            ScheduleDto(
                id = 3L,
                personId = 10L,
                medicationId = 102L,
                doseAmount = 2.0,
                doseUnit = "ml",
                doseCycle = "night",
                active = false,
                paused = false
            )
        )
        val medications = listOf(
            MedicationDto(id = 100L, name = "Amoxicillin"),
            MedicationDto(id = 101L, name = "Vitamin D"),
            MedicationDto(id = 102L, name = "Cough Syrup")
        )
        val people = listOf(PersonDto(id = 10L, name = "Alex"))

        val result = scheduler.syncSchedules(schedules, medications, people)

        assertEquals(1, result.size)
        val reminder = result.single()
        assertEquals(1L, reminder.scheduleId)
        assertEquals("Amoxicillin", reminder.medicationName)
        assertEquals("Alex", reminder.personName)
        assertEquals("500 mg", reminder.doseSummary)
        assertEquals("morning", reminder.doseCycle)
        val expectedTrigger = ZonedDateTime.of(2026, 6, 15, 8, 0, 0, 0, ZoneOffset.UTC).toInstant().toEpochMilli()
        assertEquals(expectedTrigger, reminder.triggerAtMillis)
        assertEquals(1, backend.scheduled.size)
    }

    @Test
    fun syncOccurrencesSkipsResolvedOccurrencesAndNonDoseDays() {
        val backend = FakeReminderAlarmBackend()
        val fixedNow = ZonedDateTime.of(2026, 6, 15, 7, 0, 0, 0, ZoneOffset.UTC)
        val store = InMemoryReminderOccurrenceStore()
        val scheduler = ReminderScheduler(backend, Clock.fixed(fixedNow.toInstant(), ZoneOffset.UTC), store)

        val weeklySchedule = ScheduleDto(
            id = 10L,
            personId = 1L,
            medicationId = 100L,
            doseAmount = 50.0,
            doseUnit = "mg",
            doseCycle = "morning",
            active = true,
            paused = false
        )
        val noDoseDaySchedule = ScheduleDto(
            id = 11L,
            personId = 1L,
            medicationId = 101L,
            doseAmount = 1.0,
            doseUnit = "tablet",
            doseCycle = "evening",
            active = true,
            paused = false
        )
        val occurrences = listOf(
            // Already taken today -> must be skipped
            DoseOccurrenceDto(
                key = "occ-taken",
                scheduleId = 10L,
                outcome = "taken",
                due = false,
                windowStartsOn = "2026-06-15",
                scheduledAt = "2026-06-15T08:00:00Z"
            ),
            // Already not_taken -> must be skipped
            DoseOccurrenceDto(
                key = "occ-not-taken",
                scheduleId = 10L,
                outcome = "not_taken",
                due = false,
                windowStartsOn = "2026-06-16",
                scheduledAt = "2026-06-16T08:00:00Z"
            ),
            // Next actual open occurrence on June 18 (skips June 17 non-dose day!)
            DoseOccurrenceDto(
                key = "occ-open-timed",
                scheduleId = 10L,
                outcome = "open",
                due = false,
                windowStartsOn = "2026-06-18",
                scheduledAt = "2026-06-18T09:30:00Z"
            )
            // Note: schedule 11 has NO projected occurrences in this window -> must NOT be scheduled
        )
        val medications = listOf(
            MedicationDto(id = 100L, name = "Methotrexate"),
            MedicationDto(id = 101L, name = "Weekly Vitamin D")
        )
        val people = listOf(PersonDto(id = 1L, name = "Sam"))

        val armed = scheduler.syncOccurrences(
            occurrences = occurrences,
            schedules = listOf(weeklySchedule, noDoseDaySchedule),
            medications = medications,
            people = people
        )

        assertEquals(1, armed.size)
        val reminder = armed.single()
        assertEquals(10L, reminder.scheduleId)
        assertEquals("occ-open-timed", reminder.occurrenceKey)
        assertEquals("", reminder.doseSummary)
        assertEquals(Instant.parse("2026-06-18T09:30:00Z").toEpochMilli(), reminder.triggerAtMillis)
        assertTrue(backend.scheduled.none { it.scheduleId == 11L })
    }

    @Test
    fun syncOccurrencesUsesConfiguredWindowForUntimedOccurrences() {
        val backend = FakeReminderAlarmBackend()
        val fixedNow = ZonedDateTime.of(2026, 6, 15, 7, 0, 0, 0, ZoneOffset.UTC)
        val scheduler = ReminderScheduler(backend, Clock.fixed(fixedNow.toInstant(), ZoneOffset.UTC))

        val schedule = ScheduleDto(
            id = 20L,
            medicationId = 200L,
            doseAmount = 1.0,
            doseUnit = "pill",
            doseCycle = "evening",
            active = true
        )
        val occurrences = listOf(
            DoseOccurrenceDto(
                key = "occ-untimed",
                scheduleId = 20L,
                outcome = "open",
                due = true,
                windowStartsOn = "2026-06-17",
                scheduledAt = null
            )
        )
        val customTimes = ReminderTimes.fromStrings(
            morning = "07:30:00",
            afternoon = "13:00:00",
            evening = "19:45:00",
            night = "22:00:00"
        )

        val armed = scheduler.syncOccurrences(
            occurrences = occurrences,
            schedules = listOf(schedule),
            medications = listOf(MedicationDto(id = 200L, name = "Atorvastatin")),
            people = emptyList(),
            customTimes = customTimes
        )

        val expectedMillis = ZonedDateTime.of(2026, 6, 17, 19, 45, 0, 0, ZoneOffset.UTC).toInstant().toEpochMilli()
        assertEquals(expectedMillis, armed.single().triggerAtMillis)
    }

    @Test
    fun deliveringAlarm1SchedulesAlarm2FromPersistedStoreWithoutMainActivity() {
        val storeFile = File(tempFolder.root, FileReminderOccurrenceStore.OCCURRENCES_FILE_NAME)
        val backend1 = FakeReminderAlarmBackend()
        val initialNow = Instant.parse("2026-06-15T06:00:00Z")
        val scheduler1 = ReminderScheduler(
            backend = backend1,
            clock = Clock.fixed(initialNow, ZoneOffset.UTC),
            store = FileReminderOccurrenceStore(storeFile)
        )

        val schedule = ScheduleDto(id = 30L, medicationId = 300L, doseCycle = "morning", active = true)
        val occurrences = listOf(
            DoseOccurrenceDto(
                key = "occ-1",
                scheduleId = 30L,
                outcome = "open",
                due = true,
                windowStartsOn = "2026-06-15",
                scheduledAt = "2026-06-15T08:00:00Z"
            ),
            DoseOccurrenceDto(
                key = "occ-2",
                scheduleId = 30L,
                outcome = "open",
                due = false,
                windowStartsOn = "2026-06-17",
                scheduledAt = "2026-06-17T08:00:00Z"
            )
        )

        val armed = scheduler1.syncOccurrences(
            occurrences = occurrences,
            schedules = listOf(schedule),
            medications = listOf(MedicationDto(id = 300L, name = "Levothyroxine")),
            people = emptyList()
        )
        assertEquals("occ-1", armed.single().occurrenceKey)
        assertTrue(storeFile.exists())

        // Simulate app closed & BroadcastReceiver waking at 08:00 on June 15 with a fresh scheduler reading disk
        val backend2 = FakeReminderAlarmBackend()
        val fireTime = Instant.parse("2026-06-15T08:00:00Z")
        val receiverScheduler = ReminderScheduler(
            backend = backend2,
            clock = Clock.fixed(fireTime, ZoneOffset.UTC),
            store = FileReminderOccurrenceStore(storeFile)
        )

        val next = ReminderBroadcastReceiver.handleReminderDelivery(
            scheduleId = 30L,
            occurrenceKey = "occ-1",
            triggerAtMillis = fireTime.toEpochMilli(),
            scheduler = receiverScheduler
        )

        assertNotNull(next)
        assertEquals("occ-2", next!!.occurrenceKey)
        assertEquals(Instant.parse("2026-06-17T08:00:00Z").toEpochMilli(), next.triggerAtMillis)
        assertEquals(1, backend2.scheduled.size)
        assertEquals("occ-2", backend2.scheduled.single().occurrenceKey)

        // Delivering occ-2 when no further occurrences exist returns null cleanly
        val afterSecond = ReminderBroadcastReceiver.handleReminderDelivery(
            scheduleId = 30L,
            occurrenceKey = "occ-2",
            triggerAtMillis = Instant.parse("2026-06-17T08:00:00Z").toEpochMilli(),
            scheduler = ReminderScheduler(
                backend = FakeReminderAlarmBackend(),
                clock = Clock.fixed(Instant.parse("2026-06-17T08:00:00Z"), ZoneOffset.UTC),
                store = FileReminderOccurrenceStore(storeFile)
            )
        )
        assertNull(afterSecond)
    }

    @Test
    fun syncSchedulesCancelsRemovedOrPausedSchedules() {
        val backend = FakeReminderAlarmBackend()
        val fixedNow = Instant.parse("2026-06-15T06:00:00Z")
        val scheduler = ReminderScheduler(backend, Clock.fixed(fixedNow, ZoneOffset.UTC))

        val activeSchedule = ScheduleDto(id = 11L, medicationId = 1L, doseCycle = "morning", active = true, paused = false)
        val medications = listOf(MedicationDto(id = 1L, name = "Ibuprofen"))

        scheduler.syncSchedules(listOf(activeSchedule), medications, emptyList())
        assertEquals(1, backend.scheduled.size)

        scheduler.syncSchedules(listOf(activeSchedule.copy(paused = true)), medications, emptyList())
        assertTrue(backend.scheduled.isEmpty())
        assertTrue(backend.cancelledIds.contains(11L))
    }

    @Test
    fun syncSchedulesWhenDisabledCancelsAllPreviouslyScheduledReminders() {
        val backend = FakeReminderAlarmBackend()
        val fixedNow = Instant.parse("2026-06-15T06:00:00Z")
        val scheduler = ReminderScheduler(backend, Clock.fixed(fixedNow, ZoneOffset.UTC))

        val schedules = listOf(
            ScheduleDto(id = 21L, medicationId = 1L, doseCycle = "morning", active = true),
            ScheduleDto(id = 22L, medicationId = 1L, doseCycle = "evening", active = true)
        )
        val medications = listOf(MedicationDto(id = 1L, name = "Metformin"))

        scheduler.syncSchedules(schedules, medications, emptyList(), remindersEnabled = true)
        assertEquals(2, backend.scheduled.size)

        val disabledResult = scheduler.syncSchedules(schedules, medications, emptyList(), remindersEnabled = false)
        assertTrue(disabledResult.isEmpty())
        assertTrue(backend.scheduled.isEmpty())
        assertEquals(setOf(21L, 22L), backend.cancelledIds.toSet())
    }

    @Test
    fun customReminderTimesOverrideDefaults() {
        val customTimes = ReminderTimes.fromStrings(
            morning = "07:15:00",
            afternoon = "12:45",
            evening = null,
            night = "22:30:00"
        )

        assertEquals(LocalTime.of(7, 15), customTimes.morning)
        assertEquals(LocalTime.of(12, 45), customTimes.afternoon)
        assertEquals(ReminderTimes.DEFAULT.evening, customTimes.evening)
        assertEquals(LocalTime.of(22, 30), customTimes.night)
    }

    @Test
    fun incompleteRefreshPreservesOwnedAlarmsAndAppliesPrivacy() {
        val owner = SessionOwner("https://one.example/", 1, 2)
        val store = InMemoryReminderOccurrenceStore()
        val backend = FakeReminderAlarmBackend()
        val clock = Clock.fixed(Instant.parse("2026-06-15T06:00:00Z"), ZoneOffset.UTC)
        val schedules = listOf(ScheduleDto(id = 30, medicationId = 300, active = true, doseAmount = 5.0))
        val occurrences = listOf("2026-06-15", "2026-06-17").map { date ->
            DoseOccurrenceDto(date, 30, "open", false, scheduledAt = date + "T08:00:00Z")
        }
        fun refresh(scope: SessionOwner, failed: Set<Long>?, privateText: Boolean,
                    rows: List<DoseOccurrenceDto> = emptyList(), confirmed: Boolean = true) {
            ReminderScheduler(backend, clock, store).syncOccurrences(
                rows, schedules, listOf(MedicationDto(id = 300, name = "Example")), emptyList(),
                owner = scope, failedScheduleIds = failed, privateText = privateText, preferencesConfirmed = confirmed
            )
        }
        refresh(owner, emptySet(), false, occurrences, confirmed = false)
        assertTrue(backend.scheduled.isEmpty())
        refresh(owner, emptySet(), false, occurrences)
        assertEquals(2, store.load().size)
        assertTrue(store.load().all { it.medicationName == "Example" && it.doseSummary.isEmpty() })
        refresh(owner, null, true)
        assertEquals(2, store.load().size)
        assertTrue(store.load().all { it.medicationName == "Medication" && it.personName == null && it.doseSummary.isEmpty() })
        refresh(owner, setOf(30), true)
        assertEquals(2, store.load().size)
        refresh(owner, emptySet(), true)
        assertTrue(backend.scheduled.isEmpty())
        refresh(owner, emptySet(), false, occurrences)
        refresh(owner.copy(accountId = 3), null, false)
        assertTrue(store.load().isEmpty())
    }
}
