package io.damacus.medtracker

import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import io.damacus.medtracker.auth.OidcAuthRoute
import io.damacus.medtracker.auth.HouseholdSelectionRoute
import io.damacus.medtracker.data.model.AuthenticationResult
import io.damacus.medtracker.data.model.HouseholdChoice
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class AuthenticationUiTest {
    @get:Rule val compose = createComposeRule()

    @Test fun instanceSelectionStartsBrowserLoginForTheEnteredInstance() {
        var selected: String? = null
        compose.setContent {
            MaterialTheme {
                OidcAuthRoute("Sign in to MedTracker", false, null) { selected = it }
            }
        }
        compose.onNodeWithText("Instance URL").performTextReplacement("https://my-instance.example/")
        compose.onNodeWithText("Sign in with MedTracker").performClick()
        assertEquals("https://my-instance.example/", selected)
    }

    @Test fun householdSelectionListsChoicesAndSelectsOne() {
        var selected: Long? = null
        val selection = AuthenticationResult.HouseholdSelection(
            "selection-token",
            listOf(
                HouseholdChoice(42, "Summer house", "member"),
                HouseholdChoice(43, "Winter house", "member")
            )
        )
        compose.setContent {
            MaterialTheme {
                HouseholdSelectionRoute(selection, false, null) { selected = it }
            }
        }

        compose.onNodeWithText("Choose a household").assertIsDisplayed()
        compose.onNodeWithText("Summer house").assertIsDisplayed()
        compose.onNodeWithText("Winter house").performClick()
        assertEquals(43L, selected)
    }
}
