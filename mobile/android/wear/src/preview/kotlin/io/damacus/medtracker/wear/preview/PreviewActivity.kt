package io.damacus.medtracker.wear.preview

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.navigation3.runtime.NavKey
import androidx.navigation3.runtime.entryProvider
import androidx.navigation3.runtime.rememberNavBackStack
import androidx.navigation3.ui.NavDisplay
import androidx.wear.compose.foundation.lazy.TransformingLazyColumn
import androidx.wear.compose.foundation.lazy.items
import androidx.wear.compose.foundation.lazy.rememberTransformingLazyColumnState
import androidx.wear.compose.material3.AppScaffold
import androidx.wear.compose.material3.Button
import androidx.wear.compose.material3.ButtonDefaults
import androidx.wear.compose.material3.ColorScheme
import androidx.wear.compose.material3.ListHeader
import androidx.wear.compose.material3.MaterialTheme
import androidx.wear.compose.material3.OutlinedButton
import androidx.wear.compose.material3.ScreenScaffold
import androidx.wear.compose.material3.SurfaceTransformation
import androidx.wear.compose.material3.Text
import androidx.wear.compose.material3.lazy.rememberTransformationSpec
import androidx.wear.compose.material3.lazy.transformedHeight
import androidx.wear.compose.navigation3.rememberSwipeDismissableSceneStrategy
import kotlinx.serialization.Serializable

@Serializable private data object Today : NavKey
@Serializable private data class Dose(val id: Int) : NavKey
@Serializable private data class Recorded(val id: Int) : NavKey
@Serializable private data object About : NavKey

class PreviewActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { PreviewApp() }
    }
}

@Composable
private fun PreviewApp() {
    var savedRecorded by rememberSaveable { mutableStateOf(intArrayOf()) }
    val session = remember { DemoDoseSession(savedRecorded.toSet()) }
    val recorded by session.recorded.collectAsState()
    val backStack = rememberNavBackStack(Today)
    val colours = ColorScheme(primary = Color(0xFF00E0CA), onPrimary = Color.Black,
        surfaceContainer = Color(0xFF1C2221), onSurface = Color.White,
        onSurfaceVariant = Color(0xFFBAC5C1))
    MaterialTheme(colorScheme = colours) {
        AppScaffold(timeText = {}) {
            NavDisplay(
                backStack = backStack,
                onBack = { if (backStack.size > 1) backStack.removeLastOrNull() },
                sceneStrategies = listOf(rememberSwipeDismissableSceneStrategy()),
                entryProvider = entryProvider {
                    entry<Today> {
                        TodayScreen(session.doses, recorded,
                            onDose = { backStack.add(Dose(it)) }, onAbout = { backStack.add(About) })
                    }
                    entry<Dose> { route ->
                        val dose = session.doses.first { it.id == route.id }
                        DoseScreen(dose, route.id in recorded,
                            onTake = {
                                if (session.record(route.id)) {
                                    savedRecorded = session.recorded.value.toIntArray()
                                    backStack.removeLastOrNull()
                                    backStack.add(Recorded(route.id))
                                }
                            },
                            onLater = { backStack.removeLastOrNull() })
                    }
                    entry<Recorded> { route ->
                        MessageScreen("Sample recorded", session.doses.first { it.id == route.id }.name,
                            "Demo only. No health record changed.", "Back to Today") {
                            backStack.removeLastOrNull()
                        }
                    }
                    entry<About> {
                        MessageScreen("Design preview", "Sample data only",
                            "Explore the flow. Swipe right to go back. Use the crown to scroll.", "Back to Today") {
                            backStack.removeLastOrNull()
                        }
                    }
                }
            )
        }
    }
}

@Composable
private fun TodayScreen(doses: List<DemoDose>, recorded: Set<Int>, onDose: (Int) -> Unit, onAbout: () -> Unit) {
    val state = rememberTransformingLazyColumnState()
    val transformation = rememberTransformationSpec()
    ScreenScaffold(scrollState = state) { padding ->
        TransformingLazyColumn(state = state, contentPadding = padding, modifier = Modifier.fillMaxSize()) {
            item {
                ListHeader(modifier = Modifier.transformedHeight(this, transformation),
                    transformation = SurfaceTransformation(transformation)) { Text("Today · Demo") }
            }
            items(doses, key = { it.id }) { dose ->
                Button(
                    onClick = { onDose(dose.id) },
                    modifier = Modifier.fillMaxWidth().transformedHeight(this, transformation)
                        .minimumVerticalContentPadding(ButtonDefaults.minimumVerticalListContentPadding),
                    transformation = SurfaceTransformation(transformation),
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.surfaceContainer,
                        contentColor = MaterialTheme.colorScheme.onSurface),
                    secondaryLabel = { Text(if (dose.id in recorded) "Sample recorded" else "${dose.amount} · ${dose.time}") },
                    label = { Text(dose.name) }
                )
            }
            item {
                OutlinedButton(onClick = onAbout, modifier = Modifier.fillMaxWidth()
                    .transformedHeight(this, transformation), transformation = SurfaceTransformation(transformation),
                    label = { Text("About preview") })
            }
        }
    }
}

@Composable
private fun DoseScreen(dose: DemoDose, recorded: Boolean, onTake: () -> Unit, onLater: () -> Unit) {
    val state = rememberTransformingLazyColumnState()
    ScreenScaffold(scrollState = state, contentPadding = PaddingValues(horizontal = 20.dp, vertical = 22.dp)) { padding ->
        TransformingLazyColumn(state = state, contentPadding = padding, modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            item {
                Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Next dose · Demo", style = MaterialTheme.typography.labelSmall)
                    Text(dose.name, style = MaterialTheme.typography.titleMedium, textAlign = TextAlign.Center)
                    Text("${dose.amount} · ${dose.time}", style = MaterialTheme.typography.bodyMedium)
                }
            }
            item {
                PressPill(if (recorded) "Sample recorded" else "Take dose", !recorded, onTake)
            }
            item {
                Button(onClick = onLater, modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = Color.Transparent,
                        contentColor = MaterialTheme.colorScheme.onSurfaceVariant),
                    label = { Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) { Text("Later") } })
            }
        }
    }
}

@Composable
private fun PressPill(label: String, enabled: Boolean, onClick: () -> Unit) {
    val interactions = remember { MutableInteractionSource() }
    val pressed by interactions.collectIsPressedAsState()
    val colours = MaterialTheme.colorScheme
    OutlinedButton(onClick = onClick, enabled = enabled, interactionSource = interactions,
        modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp),
        border = BorderStroke(1.5.dp, if (enabled) colours.primary else colours.onSurfaceVariant),
        colors = ButtonDefaults.buttonColors(
            containerColor = if (pressed) colours.primary else Color.Transparent,
            contentColor = if (pressed) colours.onPrimary else colours.primary),
        label = { Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) { Text(label) } })
}

@Composable
private fun MessageScreen(title: String, name: String, body: String, action: String, onClick: () -> Unit) {
    val state = rememberTransformingLazyColumnState()
    ScreenScaffold(scrollState = state) { padding ->
        TransformingLazyColumn(state = state, contentPadding = padding, modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            item { Text(title, style = MaterialTheme.typography.titleMedium, textAlign = TextAlign.Center) }
            item { Text(name, style = MaterialTheme.typography.bodyMedium, textAlign = TextAlign.Center) }
            item { Text(body, modifier = Modifier.padding(horizontal = 20.dp),
                style = MaterialTheme.typography.bodySmall, textAlign = TextAlign.Center) }
            item { PressPill(action, true, onClick) }
        }
    }
}
