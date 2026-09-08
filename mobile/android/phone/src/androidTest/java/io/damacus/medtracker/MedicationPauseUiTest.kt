package io.damacus.medtracker

import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import android.os.ParcelFileDescriptor
import io.damacus.medtracker.data.model.*
import io.damacus.medtracker.ui.dashboard.*
import org.junit.Rule
import org.junit.Test

class MedicationPauseUiTest {
    @get:Rule val compose = createComposeRule()

    @Test fun failedPauseKeepsReasonAndNoteVisible() {
        compose.setContent {
            MaterialTheme {
                PauseEditor(
                    source = PauseSource("schedule", "id", 1, "Weekly medicine", false),
                    form = PauseForm(PauseReason.OTHER, "Keep this note", error = "Connect to the internet and try again."),
                    onReason = {}, onNote = {}, onSubmit = {}, onDismiss = {}
                )
            }
        }
        compose.onNodeWithText("Keep this note").assertIsDisplayed()
        compose.onNodeWithText("Connect to the internet and try again.").assertIsDisplayed()
        compose.onNodeWithText("Confirm pause").assertIsEnabled()
        val capture = InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand("screencap -p /sdcard/Download/medication-pause-phone.png")
        ParcelFileDescriptor.AutoCloseInputStream(capture).use { it.readBytes() }
    }

    @Test fun noReasonDisablesConfirmation() {
        compose.setContent {
            MaterialTheme {
                PauseEditor(PauseSource("person_medication", "id", 1, "PRN medicine", false), PauseForm(), {}, {}, {}, {})
            }
        }
        compose.onNodeWithText("Confirm pause").assertIsNotEnabled()
        compose.onNodeWithText("Note (optional)").assertIsDisplayed()
    }

    @Test fun pausedCardShowsCurrentReasonAndNoteWithoutOpeningHistory() {
        compose.setContent {
            MaterialTheme {
                PausedTreatmentDetails(PauseSource("schedule", "id", 1, "Medicine", true, currentReason = "side_effects", currentNote = "Discuss at review"))
            }
        }
        compose.onNodeWithText("Side effects").assertIsDisplayed()
        compose.onNodeWithText("Discuss at review").assertIsDisplayed()
    }
}
