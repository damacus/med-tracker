package io.damacus.medtracker.data

import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SessionManagerTest {

    private class InMemoryCredentialStore : CredentialStore {
        var stored: String? = null
        override fun read(): String? = stored
        override fun write(value: String) {
            stored = value
        }
        override fun clear() {
            stored = null
        }
    }

    @Test
    fun updatePayloadPersistsUserProfileWithoutChangingSessionRevision() {
        val store = InMemoryCredentialStore()
        val manager = SessionManager(store)
        manager.saveSession(
            SessionPayload(
                accessToken = "token-1",
                refreshToken = "refresh-1",
                household = HouseholdDto(7L, "Home"),
                me = null
            ),
            serverUrl = "http://10.0.2.2:5150"
        )
        val initialRevision = manager.sessionState.value.revision
        var observerCalls = 0
        val subscription = manager.observeSession { observerCalls++ }
        assertEquals(1, observerCalls)

        val user = UserDto(id = 42L, emailAddress = "alice@example.com", name = "Alice Example", role = "administrator")
        val updated = manager.updatePayload(
            initialRevision,
            manager.sessionState.value.sessionPayload!!.copy(me = user)
        )

        assertTrue(updated)
        assertEquals(initialRevision, manager.sessionState.value.revision)
        assertEquals("http://10.0.2.2:5150/", manager.sessionState.value.serverUrl)
        assertEquals(user, manager.sessionState.value.user)
        assertEquals(1, observerCalls)

        val reloaded = SessionManager(store)
        assertEquals(user, reloaded.sessionState.value.user)
        assertEquals(7L, reloaded.sessionState.value.household?.id)
        subscription.close()
    }

    @Test
    fun updatePayloadRejectsStaleRevisionAfterLogout() {
        val store = InMemoryCredentialStore()
        val manager = SessionManager(store)
        manager.saveSession(
            SessionPayload(
                accessToken = "token-1",
                refreshToken = "refresh-1",
                household = HouseholdDto(7L, "Home"),
                me = null
            ),
            serverUrl = "http://10.0.2.2:5150/"
        )
        val staleRevision = manager.sessionState.value.revision
        manager.clearSession()

        val updated = manager.updatePayload(
            staleRevision,
            SessionPayload(
                accessToken = "token-1",
                refreshToken = "refresh-1",
                household = HouseholdDto(7L, "Home"),
                me = UserDto(1L, "a@b.c", "Alice", "owner")
            )
        )

        assertFalse(updated)
        assertNull(manager.sessionState.value.sessionPayload)
        assertNull(store.stored)
        assertNotNull(manager.sessionState.value.revision)
    }
}
