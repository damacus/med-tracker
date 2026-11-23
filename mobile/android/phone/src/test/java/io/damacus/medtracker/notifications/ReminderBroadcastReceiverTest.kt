package io.damacus.medtracker.notifications

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ReminderBroadcastReceiverTest {

    @Test
    fun buildNotificationContentIncludesMedicationDoseAndPerson() {
        val content = ReminderBroadcastReceiver.buildNotificationContent(
            medicationName = "Amoxicillin",
            personName = "Alex",
            doseSummary = "500 mg",
            doseCycle = "morning"
        )

        assertEquals("Time for Amoxicillin", content.title)
        assertTrue(content.body.contains("Alex"))
        assertTrue(content.body.contains("500 mg"))
        assertTrue(content.body.contains("morning"))
    }

    @Test
    fun buildNotificationContentHandlesMissingPersonAndDoseGracefully() {
        val content = ReminderBroadcastReceiver.buildNotificationContent(
            medicationName = "Vitamin D",
            personName = null,
            doseSummary = "",
            doseCycle = null
        )

        assertEquals("Time for Vitamin D", content.title)
        assertEquals("Scheduled dose is due now.", content.body)
    }
}
