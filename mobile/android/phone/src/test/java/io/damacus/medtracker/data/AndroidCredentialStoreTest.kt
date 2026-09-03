package io.damacus.medtracker.data

import java.io.IOException
import java.nio.file.Files
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.assertThrows
import org.junit.Test

class AndroidCredentialStoreTest {
    @Test
    fun storesOnlyEncryptedBytesUnderTheNoBackupDirectory() {
        val noBackupDirectory = Files.createTempDirectory("medtracker-no-backup").toFile()
        val store = AndroidCredentialStore(noBackupDirectory, AesGcmCipher)
        val session = "{\"accessToken\":\"access-secret\",\"refreshToken\":\"refresh-secret\"}"

        store.write(session)

        val credentialFile = noBackupDirectory.resolve("session_credentials.enc")
        assertTrue(credentialFile.isFile)
        val storedBytes = credentialFile.readBytes().toString(Charsets.UTF_8)
        assertFalse(storedBytes.contains("access-secret"))
        assertFalse(storedBytes.contains("refresh-secret"))
        assertEquals(session, store.read())

        store.clear()

        assertNull(store.read())
    }

    @Test
    fun failedAtomicReplacementPreservesThePreviousCredentialFile() {
        val noBackupDirectory = Files.createTempDirectory("medtracker-no-backup").toFile()
        val initialStore = AndroidCredentialStore(noBackupDirectory, AesGcmCipher)
        initialStore.write("previous-session-with-longer-content")
        val failingStore = AndroidCredentialStore(
            noBackupDirectory,
            AesGcmCipher,
            CredentialFileReplacer { _, _ -> throw IOException("replacement failed") }
        )

        assertThrows(IOException::class.java) {
            failingStore.write("new-session")
        }

        assertEquals("previous-session-with-longer-content", initialStore.read())
        assertTrue(noBackupDirectory.listFiles().orEmpty().none { it.name.endsWith(".tmp") })
    }

    @Test
    fun shorterCredentialReplacementLeavesNoPreviousCiphertextSuffix() {
        val noBackupDirectory = Files.createTempDirectory("medtracker-no-backup").toFile()
        val store = AndroidCredentialStore(noBackupDirectory, AesGcmCipher)

        store.write("previous-session-with-longer-content")
        val previousSize = noBackupDirectory.resolve("session_credentials.enc").length()
        store.write("new-session")

        assertEquals("new-session", store.read())
        assertTrue(noBackupDirectory.resolve("session_credentials.enc").length() < previousSize)
    }

    private object AesGcmCipher : CredentialCipher {
        private val key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()

        override fun encrypt(plaintext: ByteArray): EncryptedCredential {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key)
            return EncryptedCredential(cipher.iv, cipher.doFinal(plaintext))
        }

        override fun decrypt(value: EncryptedCredential): ByteArray {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, value.initializationVector))
            return cipher.doFinal(value.ciphertext)
        }
    }
}
