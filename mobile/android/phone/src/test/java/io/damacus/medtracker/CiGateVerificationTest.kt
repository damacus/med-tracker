package io.damacus.medtracker

import org.junit.Assert.fail
import org.junit.Test

class CiGateVerificationTest {
    @Test
    fun requiredAndroidFailureBlocksTheGate() {
        fail("Deliberate CI verification failure; remove after verification")
    }
}
