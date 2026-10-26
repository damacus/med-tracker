package io.damacus.medtracker.wear.preview

import android.os.SystemClock
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.wear.compose.material3.ButtonDefaults
import androidx.wear.compose.material3.MaterialTheme
import androidx.wear.compose.material3.OutlinedButton
import androidx.wear.compose.material3.Text

@Composable
internal fun HoldToTakePill(enabled: Boolean, active: Boolean, onComplete: () -> Unit) {
    val hold = remember { HoldToTake() }
    val progress = remember { mutableFloatStateOf(0f) }
    val interactions = remember { MutableInteractionSource() }
    val complete by rememberUpdatedState(onComplete)
    val available by rememberUpdatedState(enabled && active)
    var pressed by remember { mutableStateOf(false) }
    var pressedAt by remember { mutableLongStateOf(0L) }
    var buttonWidth by remember { mutableIntStateOf(0) }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    var resumed by remember(lifecycle) { mutableStateOf(lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)) }
    DisposableEffect(lifecycle) {
        val observer = LifecycleEventObserver { _, _ ->
            resumed = lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)
            if (!resumed) hold.cancel()
        }
        lifecycle.addObserver(observer)
        onDispose {
            lifecycle.removeObserver(observer)
            hold.cancel()
            progress.floatValue = 0f
        }
    }
    LaunchedEffect(pressed, enabled, active, resumed) {
        if (!pressed || !enabled || !active || !resumed) {
            hold.cancel()
            if (active && resumed) {
                animate(progress.floatValue, 0f, animationSpec = tween(150)) { value, _ -> progress.floatValue = value }
            } else {
                progress.floatValue = 0f
            }
            return@LaunchedEffect
        }
        if (!hold.start(pressedAt)) return@LaunchedEffect
        try {
            while (true) {
                val frame = withFrameNanos {
                    if (!pressed || !available || !resumed) {
                        hold.cancel()
                    }
                    hold.advance(SystemClock.uptimeMillis() * 1_000_000)
                }
                progress.floatValue = frame.progress
                if (frame.completedNow) {
                    complete()
                    break
                }
            }
        } finally {
            hold.cancel()
        }
    }
    val scale by animateFloatAsState(if (pressed) 0.97f else 1f,
        spring(dampingRatio = Spring.DampingRatioNoBouncy, stiffness = Spring.StiffnessMedium), label = "Hold press")
    val accent = MaterialTheme.colorScheme.primary
    val label = if (!enabled) "Sample recorded" else if (pressed) "Keep holding…" else "Take dose"
    OutlinedButton(
        enabled = enabled && active && resumed,
        onClick = { if (active && resumed && hold.confirmAccessible()) complete() },
        interactionSource = interactions,
        shape = CircleShape,
        border = BorderStroke(1.5.dp, if (enabled) accent else MaterialTheme.colorScheme.onSurfaceVariant),
        colors = ButtonDefaults.buttonColors(containerColor = Color.Transparent, contentColor = accent),
        modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp)
            .onSizeChanged { buttonWidth = it.width }
            .graphicsLayer { scaleX = scale; scaleY = scale }
            .drawWithCache {
                val path = Path().apply { addRoundRect(RoundRect(0f, 0f, size.width, size.height, CornerRadius(size.height / 2))) }
                onDrawBehind {
                    clipPath(path) {
                        clipRect(right = size.width * progress.floatValue) { drawRect(accent) }
                    }
                }
            }
            .semantics { stateDescription = "Hold for one second to record this sample dose" }
            .pointerInput(enabled, active, resumed) {
                if (!enabled || !active || !resumed) return@pointerInput
                awaitEachGesture {
                    val down = awaitFirstDown(pass = PointerEventPass.Initial)
                    down.consume()
                    val interaction = PressInteraction.Press(down.position)
                    interactions.tryEmit(interaction)
                    pressedAt = SystemClock.uptimeMillis() * 1_000_000
                    pressed = true
                    try {
                        while (true) {
                            val event = awaitPointerEvent(PointerEventPass.Initial)
                            val change = event.changes.firstOrNull { it.id == down.id } ?: break
                            val outside = change.position.x !in 0f..size.width.toFloat() ||
                                change.position.y !in 0f..size.height.toFloat()
                            val cancelled = outside || change.isConsumed || event.changes.any { it.id != down.id && it.pressed }
                            change.consume()
                            if (cancelled || !change.pressed) break
                        }
                    } finally {
                        hold.cancel()
                        pressed = false
                        interactions.tryEmit(PressInteraction.Cancel(interaction))
                    }
                }
            },
        label = {
            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
                Text(label)
                Box(Modifier.fillMaxWidth().clearAndSetSemantics { }.drawWithContent {
                    val inset = (buttonWidth - size.width) / 2f
                    clipRect(right = (buttonWidth * progress.floatValue - inset).coerceIn(0f, size.width)) { this@drawWithContent.drawContent() }
                }, contentAlignment = Alignment.Center) { Text(label, color = Color.Black) }
            }
        }
    )
}
