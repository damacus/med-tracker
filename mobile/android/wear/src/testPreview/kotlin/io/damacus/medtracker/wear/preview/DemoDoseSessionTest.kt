package io.damacus.medtracker.wear.preview

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DemoDoseSessionTest {
    @Test fun recordingOneSampleDoesNotChangeAnother() {
        val session = DemoDoseSession()
        assertTrue(session.record(1))
        assertEquals(setOf(1), session.recorded.value)
        assertFalse(session.recorded.value.contains(2))
    }

    @Test fun duplicateActivationDoesNotRecordAgain() {
        val session = DemoDoseSession()
        assertTrue(session.record(1))
        assertFalse(session.record(1))
        assertEquals(setOf(1), session.recorded.value)
    }

    @Test fun unknownDoseCannotBeRecorded() {
        val session = DemoDoseSession()
        assertFalse(session.record(999))
        assertTrue(session.recorded.value.isEmpty())
    }

    @Test fun openingAndLeavingSampleDosesDoesNotRecordThem() {
        val session = DemoDoseSession()
        assertEquals("Vitamin D", session.doses.first().name)
        assertTrue(session.recorded.value.isEmpty())
    }
}
