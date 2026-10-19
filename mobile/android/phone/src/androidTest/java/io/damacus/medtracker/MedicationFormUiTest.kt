package io.damacus.medtracker

import android.graphics.Bitmap
import androidx.test.espresso.Espresso
import androidx.test.platform.app.InstrumentationRegistry
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.runtime.mutableStateOf
import io.damacus.medtracker.data.model.LocationDto
import io.damacus.medtracker.ui.medication.MedicationFormScreen
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import java.io.File

class MedicationFormUiTest {
    @get:Rule val compose = createComposeRule()

    @Test fun untouchedLocationSubmitsNoStorageLabel() {
        val locations = mutableListOf<Int?>()
        compose.setContent {
            MaterialTheme {
                MedicationFormScreen(onBackClick = {}, onSubmit = { _, location -> locations.add(location) })
            }
        }
        compose.onNodeWithText("Medication Name *").performTextReplacement("Synthetic unlabelled medicine")
        compose.onNodeWithText("Save Medication").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf<Int?>(null), locations) }
        captureForm("loco-android-medication-form-no-label.png")
    }

    @Test fun namedStorageLabelCanBeSelectedAndCleared() {
        val submitted = mutableListOf<Int?>()
        compose.setContent {
            MaterialTheme {
                MedicationFormScreen(onBackClick = {}, onSubmit = { _, label -> submitted.add(label) },
                    locations = listOf(LocationDto(id = 7L, name = "Kitchen cupboard")))
            }
        }
        compose.onNodeWithText("Medication Name *").performTextReplacement("Synthetic medicine")
        compose.onNodeWithText("No storage label").performScrollTo().performClick()
        compose.onNodeWithText("Kitchen cupboard").performClick()
        compose.onNodeWithText("Save Medication").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf(7), submitted) }
        captureForm("loco-android-medication-form-named-label.png")
        compose.onNodeWithText("Kitchen cupboard").performScrollTo().performClick()
        compose.onNodeWithText("No storage label").performClick()
        compose.onNodeWithText("Save Medication").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf(7, null), submitted) }
    }

    @Test fun householdRevisionClearsFormAndPreviousLabelChoice() {
        val revision = mutableStateOf("first-household")
        val submitted = mutableListOf<Int?>()
        compose.setContent {
            MaterialTheme {
                MedicationFormScreen(onBackClick = {}, onSubmit = { _, label -> submitted.add(label) },
                    sessionRevision = revision.value,
                    locations = if (revision.value == "first-household") listOf(LocationDto(id = 7L, name = "Previous household cupboard")) else emptyList())
            }
        }
        compose.onNodeWithText("Medication Name *").performTextReplacement("Previous household medicine")
        compose.onNodeWithText("No storage label").performScrollTo().performClick()
        compose.onNodeWithText("Previous household cupboard").performClick()
        compose.runOnIdle { revision.value = "second-household" }
        compose.onNodeWithText("Previous household medicine").assertDoesNotExist()
        compose.onNodeWithText("Previous household cupboard").assertDoesNotExist()
        compose.onNodeWithText("Medication Name *").performScrollTo().performTextReplacement("Current household medicine")
        compose.onNodeWithText("Save Medication").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf<Int?>(null), submitted) }
    }

    private fun captureForm(filename: String) {
        Espresso.closeSoftKeyboard()
        compose.onNodeWithText("Medication Name *").performScrollTo()
        compose.waitForIdle()
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val screenshot = checkNotNull(instrumentation.uiAutomation.takeScreenshot())
        val outputPath = InstrumentationRegistry.getArguments().getString("additionalTestOutputDir")
            ?: "/sdcard/Android/media/${instrumentation.targetContext.packageName}/additional_test_output"
        val directory = File(outputPath)
        check(directory.isDirectory || directory.mkdirs())
        File(directory, filename).outputStream().use { output ->
            check(screenshot.compress(Bitmap.CompressFormat.PNG, 100, output))
        }
        screenshot.recycle()
    }
}
