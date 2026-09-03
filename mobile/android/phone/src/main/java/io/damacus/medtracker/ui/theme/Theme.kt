package io.damacus.medtracker.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.unit.dp

private val LightColorScheme = lightColorScheme(
    primary = MedTrackerPrimary,
    onPrimary = MedTrackerOnPrimary,
    primaryContainer = MedTrackerPrimaryContainer,
    onPrimaryContainer = MedTrackerOnPrimaryContainer,
    secondary = MedTrackerSecondary,
    onSecondary = MedTrackerOnSecondary,
    secondaryContainer = MedTrackerSecondaryContainer,
    onSecondaryContainer = MedTrackerOnSecondaryContainer,
    background = MedTrackerBackground,
    onBackground = MedTrackerOnBackground,
    surface = MedTrackerSurface,
    onSurface = MedTrackerOnSurface,
    surfaceVariant = MedTrackerSurfaceVariant,
    onSurfaceVariant = MedTrackerOnSurfaceVariant,
    outline = MedTrackerOutline,
    outlineVariant = MedTrackerOutlineVariant
)

private val DarkColorScheme = darkColorScheme(
    primary = MedTrackerPrimary,
    onPrimary = MedTrackerOnPrimary,
    primaryContainer = MedTrackerDarkSurfaceVariant,
    onPrimaryContainer = MedTrackerOnPrimaryContainer,
    secondary = MedTrackerSecondary,
    onSecondary = MedTrackerOnSecondary,
    secondaryContainer = MedTrackerDarkSurfaceVariant,
    onSecondaryContainer = MedTrackerOnSecondaryContainer,
    background = MedTrackerDarkBackground,
    onBackground = MedTrackerDarkOnBackground,
    surface = MedTrackerDarkSurface,
    onSurface = MedTrackerDarkOnSurface,
    surfaceVariant = MedTrackerDarkSurfaceVariant,
    onSurfaceVariant = MedTrackerDarkOnSurfaceVariant,
    outline = MedTrackerDarkOutline,
    outlineVariant = MedTrackerDarkOutline
)

val MedTrackerShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(24.dp)
)

@Composable
fun MedTrackerTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        shapes = MedTrackerShapes,
        content = content
    )
}
