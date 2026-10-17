package io.damacus.medtracker.wear

import io.damacus.medtracker.wear.protocol.*
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.common.api.Status
import com.google.android.gms.wearable.WearableStatusCodes
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test

class CompanionConnectionTest {
    @Test fun `unknown capability means missing phone app but transport errors are not hidden`() = runBlocking {
        assertEquals(emptySet<String>(), discoverCapabilityNodes {
            throw ApiException(Status(WearableStatusCodes.UNKNOWN_CAPABILITY))
        })
        try {
            discoverCapabilityNodes { throw ApiException(Status(WearableStatusCodes.API_NOT_CONNECTED)) }
            fail("Transport error must propagate to disconnected state")
        } catch (error: ApiException) {
            assertEquals(WearableStatusCodes.API_NOT_CONNECTED, error.statusCode)
        }
    }

    private val phone = setOf("phone")
    private fun payload(signedIn: Boolean) = CompanionProtocol.encode(CompanionStatus("1", if (signedIn) SessionState.SIGNED_IN else SessionState.SIGNED_OUT, 1))

    @Test fun `later snapshot removes stale cached status and converges after discovery failure`() {
        val connection = CompanionConnection()
        connection.replaceStatuses(mapOf("phone" to payload(true)))
        connection.discovery(phone, phone)
        assertEquals(ConnectionState.READY, connection.state)
        connection.discoveryFailed()
        assertEquals(ConnectionState.DISCONNECTED, connection.state)
        connection.replaceStatuses(emptyMap())
        connection.discovery(phone, phone)
        assertEquals(ConnectionState.WAITING_FOR_STATUS, connection.state)
        connection.replaceStatuses(mapOf("phone" to payload(false)))
        assertEquals(ConnectionState.SIGNED_OUT, connection.state)
    }

    @Test fun `discovery distinguishes missing disconnected and waiting`() {
        val connection = CompanionConnection()
        connection.discovery(emptySet(), emptySet())
        assertEquals(ConnectionState.PHONE_APP_MISSING, connection.state)
        connection.discovery(phone, emptySet())
        assertEquals(ConnectionState.DISCONNECTED, connection.state)
        connection.discovery(phone, phone)
        assertEquals(ConnectionState.WAITING_FOR_STATUS, connection.state)
        connection.discoveryFailed()
        assertEquals(ConnectionState.DISCONNECTED, connection.state)
    }

    @Test fun `persistent status converges before and after reconnect`() {
        val connection = CompanionConnection()
        connection.status("phone", payload(true))
        connection.discovery(phone, phone)
        assertEquals(ConnectionState.READY, connection.state)
        connection.discovery(phone, emptySet())
        assertEquals(ConnectionState.DISCONNECTED, connection.state)
        connection.status("phone", payload(false))
        assertEquals(ConnectionState.DISCONNECTED, connection.state)
        connection.discovery(phone, phone)
        assertEquals(ConnectionState.SIGNED_OUT, connection.state)
        connection.status("phone", payload(true))
        assertEquals(ConnectionState.READY, connection.state)
        connection.status("phone", null)
        assertEquals(ConnectionState.WAITING_FOR_STATUS, connection.state)
    }

    @Test fun `unknown nodes cannot authenticate and errors fail closed`() {
        val connection = CompanionConnection()
        connection.status("other", payload(true))
        connection.discovery(phone, phone)
        assertEquals(ConnectionState.WAITING_FOR_STATUS, connection.state)
        connection.status("phone", "{\"protocolVersion\":2}".toByteArray())
        assertEquals(ConnectionState.INCOMPATIBLE, connection.state)
        connection.status("phone", byteArrayOf(1))
        assertEquals(ConnectionState.INCOMPATIBLE, connection.state)
        connection.status("phone", payload(true))
        connection.discoveryFailed()
        assertEquals(ConnectionState.DISCONNECTED, connection.state)
        connection.discovery(emptySet(), emptySet())
        assertEquals(ConnectionState.PHONE_APP_MISSING, connection.state)
    }

    @Test fun `schedule converges with active node and WearUiSpec enforces centered round display metrics`() {
        val connection = CompanionConnection()
        val schedule = CompanionSchedule(
            people = listOf(PersonData("p-maya", "Maya (Child)", 1)),
            medications = listOf(MedicationData("m-amox", "p-maya", "Amoxicillin", "250mg • 1 capsule", "due_now", "10:00 AM")),
            publishedAt = 100L
        )
        connection.replaceStatuses(mapOf("phone" to payload(true)))
        connection.replaceSchedules(mapOf("phone" to ScheduleProtocol.encode(schedule)))
        connection.discovery(phone, phone)
        assertEquals("phone", connection.activeNode)
        assertEquals(schedule, connection.currentSchedule)

        assertEquals(android.view.Gravity.CENTER, WearUiSpec.ACTION_BUTTON_GRAVITY)
        assertEquals(3, WearUiSpec.CHIP_MAX_LINES)
        assertEquals(20, WearUiSpec.CONTAINER_HORIZONTAL_PAD_DP)
        assertEquals(36, WearUiSpec.CONTAINER_VERTICAL_PAD_DP)
        assertEquals(android.view.HapticFeedbackConstants.CONFIRM, WearUiSpec.CONFIRMATION_HAPTIC)
        assertEquals(26, WearUiSpec.CHIP_CORNER_RADIUS_DP)
        assertEquals(24, WearUiSpec.BUTTON_CORNER_RADIUS_DP)
        assertEquals(26, WearUiSpec.CARD_CORNER_RADIUS_DP)
        assertEquals(
            "250mg • 1 capsule\nDue now",
            formatMedicationSubtitle(schedule.medications.first(), "Due now", "As needed")
        )
    }
}
