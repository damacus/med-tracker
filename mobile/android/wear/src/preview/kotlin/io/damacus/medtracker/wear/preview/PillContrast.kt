package io.damacus.medtracker.wear.preview

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance

internal fun pillContentColour(background: Color, accent: Color): Color {
    val luminance = background.luminance()
    val accentLuminance = accent.luminance()
    val accentContrast = (maxOf(luminance, accentLuminance) + 0.05f) /
        (minOf(luminance, accentLuminance) + 0.05f)
    if (accentContrast >= 4.5f) return accent
    val blackContrast = (luminance + 0.05f) / 0.05f
    val whiteContrast = 1.05f / (luminance + 0.05f)
    return if (blackContrast >= whiteContrast) Color.Black else Color.White
}
