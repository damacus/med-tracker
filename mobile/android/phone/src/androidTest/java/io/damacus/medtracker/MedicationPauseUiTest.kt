package io.damacus.medtracker

import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import android.os.ParcelFileDescriptor
import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.api.GeneratedMedicationPauseGateway
import io.damacus.medtracker.data.model.*
import io.damacus.medtracker.ui.dashboard.*
import org.junit.Rule
import org.junit.Test
import kotlinx.coroutines.MainScope

class MedicationPauseUiTest {
    @get:Rule val compose = createComposeRule()

    @Test fun inactivePausedScheduleEnablesResume() = assertPausedResumeEnabled("schedule")

    @Test fun inactivePausedAssignmentEnablesResume() = assertPausedResumeEnabled("person_medication")

    private fun assertPausedResumeEnabled(type: String) {
        val source = PauseSource(type, "id", 1, "Medicine", true, currentPauseId = "period", active = false)
        val controller = MedicationPauseController(AppSession("https://example.test", null), GeneratedMedicationPauseGateway(), MainScope(), { true }, {})
        compose.setContent {
            MaterialTheme {
                MedicationPauseControls(MedicationPauseState(supported = true, sources = listOf(source)), null, controller)
            }
        }
        compose.onNodeWithText("Resume").assertIsEnabled()
    }

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
