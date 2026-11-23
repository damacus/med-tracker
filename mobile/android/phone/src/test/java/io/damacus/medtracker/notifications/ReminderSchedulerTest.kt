package io.damacus.medtracker.notifications

import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.ScheduleDto
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Clock
import java.time.Instant
import java.time.LocalTime
import java.time.ZoneOffset
import java.time.ZonedDateTime

class ReminderSchedulerTest {

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
    fun nextTriggerAdvancesToTomorrowWhenTimeHasPassed() {
        val backend = FakeReminderAlarmBackend()
        val fixedNow = ZonedDateTime.of(2026, 6, 15, 10, 30, 0, 0, ZoneOffset.UTC)
        val scheduler = ReminderScheduler(backend, Clock.fixed(fixedNow.toInstant(), ZoneOffset.UTC))

        val schedules = listOf(
            ScheduleDto(
                id = 5L,
                medicationId = 200L,
                doseAmount = 10.0,
                doseUnit = "mg",
                doseCycle = "morning",
                active = true,
                paused = false
            )
        )
        val medications = listOf(MedicationDto(id = 200L, name = "Cetirizine"))

        val result = scheduler.syncSchedules(schedules, medications, emptyList())

        val expectedTrigger = ZonedDateTime.of(2026, 6, 16, 8, 0, 0, 0, ZoneOffset.UTC).toInstant().toEpochMilli()
        assertEquals(expectedTrigger, result.single().triggerAtMillis)
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
        val backend = FakeReminderAlarmBackend()
        val fixedNow = ZonedDateTime.of(2026, 6, 15, 6, 0, 0, 0, ZoneOffset.UTC)
        val scheduler = ReminderScheduler(backend, Clock.fixed(fixedNow.toInstant(), ZoneOffset.UTC))
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

        val result = scheduler.syncSchedules(
            schedules = listOf(ScheduleDto(id = 9L, medicationId = 1L, doseCycle = "morning", active = true)),
            medications = listOf(MedicationDto(id = 1L, name = "Aspirin")),
            people = emptyList(),
            customTimes = customTimes
        )
        val expectedTrigger = ZonedDateTime.of(2026, 6, 15, 7, 15, 0, 0, ZoneOffset.UTC).toInstant().toEpochMilli()
        assertEquals(expectedTrigger, result.single().triggerAtMillis)
    }
}
