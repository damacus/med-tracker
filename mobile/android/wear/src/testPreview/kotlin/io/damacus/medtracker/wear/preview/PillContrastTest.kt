package io.damacus.medtracker.wear.preview

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PillContrastTest {
    private val accent = Color(0xFF00E0CA)

    @Test fun labelStaysReadableThroughoutTheAnimatedFillAndRelease() {
        for (frame in 0..100) {
            val background = lerp(Color.Black, accent, frame / 100f)
            val foreground = pillContentColour(background, accent)
            val light = maxOf(background.luminance(), foreground.luminance())
            val dark = minOf(background.luminance(), foreground.luminance())
            assertTrue("Unreadable label at fill frame $frame", (light + 0.05f) / (dark + 0.05f) >= 4.5f)
        }
    }

    @Test fun restingAndPressedLabelsRetainTheChosenColours() {
        assertEquals(accent, pillContentColour(Color.Black, accent))
        assertEquals(Color.Black, pillContentColour(accent, accent))
    }
}
