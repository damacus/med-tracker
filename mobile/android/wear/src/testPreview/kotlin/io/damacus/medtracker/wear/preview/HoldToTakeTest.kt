package io.damacus.medtracker.wear.preview

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HoldToTakeTest {
    @Test fun continuousHoldFillsForOneSecondAndCompletesWhileStillHeld() {
        val hold = HoldToTake()
        hold.start(0)
        assertEquals(0.25f, hold.advance(250_000_000).progress)
        assertEquals(0.75f, hold.advance(750_000_000).progress)
        assertFalse(hold.advance(999_000_000).completedNow)
        assertTrue(hold.advance(1_000_000_000).completedNow)
        assertFalse(hold.advance(2_000_000_000).completedNow)
        assertFalse(hold.start(3_000_000_000))
    }

    @Test fun releaseOrInterruptionResetsAndCannotCompleteLater() {
        val hold = HoldToTake()
        hold.start(0)
        assertEquals(0.5f, hold.advance(500_000_000).progress)
        hold.cancel()
        assertEquals(0f, hold.advance(2_000_000_000).progress)
        assertFalse(hold.advance(2_000_000_000).completedNow)
        hold.start(2_000_000_000)
        assertEquals(0.1f, hold.advance(2_100_000_000).progress)
        assertFalse(hold.advance(2_100_000_000).completedNow)
    }

    @Test fun accessibleActivationCompletesOnceWithoutAPointerHold() {
        val hold = HoldToTake()
        assertTrue(hold.confirmAccessible())
        assertFalse(hold.confirmAccessible())
        assertFalse(hold.start(0))
        assertFalse(hold.advance(2_000_000_000).completedNow)
    }

    @Test fun aCancelledHoldCanBeCompletedByAccessibleActivationOnlyOnce() {
        val hold = HoldToTake()
        hold.start(0)
        hold.cancel()
        assertTrue(hold.confirmAccessible())
        assertFalse(hold.advance(2_000_000_000).completedNow)
    }
}
