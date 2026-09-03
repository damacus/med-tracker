package io.damacus.medtracker.data

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.nio.ByteBuffer
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

data class EncryptedCredential(val initializationVector: ByteArray, val ciphertext: ByteArray)

interface CredentialCipher {
    fun encrypt(plaintext: ByteArray): EncryptedCredential
    fun decrypt(value: EncryptedCredential): ByteArray
}

private data class CredentialFile(val file: File)

fun interface CredentialFileReplacer {
    fun replace(source: File, target: File)
}

private object AtomicCredentialFileReplacer : CredentialFileReplacer {
    override fun replace(source: File, target: File) {
        try {
            Files.move(
                source.toPath(),
                target.toPath(),
                StandardCopyOption.ATOMIC_MOVE,
                StandardCopyOption.REPLACE_EXISTING
            )
        } catch (_: AtomicMoveNotSupportedException) {
            Files.move(source.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING)
        }
    }
}

class AndroidCredentialStore private constructor(
    credentialFile: CredentialFile,
    private val cipher: CredentialCipher = AndroidKeystoreCredentialCipher(),
    private val fileReplacer: CredentialFileReplacer = AtomicCredentialFileReplacer
) : CredentialStore {
    private val file = credentialFile.file

    constructor(
        context: Context,
        cipher: CredentialCipher = AndroidKeystoreCredentialCipher()
    ) : this(CredentialFile(File(context.noBackupFilesDir, FILE_NAME)), cipher, AtomicCredentialFileReplacer)

    internal constructor(
        noBackupFilesDir: File,
        cipher: CredentialCipher,
        fileReplacer: CredentialFileReplacer = AtomicCredentialFileReplacer
    ) : this(CredentialFile(File(noBackupFilesDir, FILE_NAME)), cipher, fileReplacer)

    override fun read(): String? {
        if (!file.isFile) return null
        val bytes = file.readBytes()
        if (bytes.size < Integer.BYTES) return null
        val buffer = ByteBuffer.wrap(bytes)
        val vectorLength = buffer.int
        if (vectorLength <= 0 || vectorLength > buffer.remaining()) return null
        val vector = ByteArray(vectorLength).also(buffer::get)
        val ciphertext = ByteArray(buffer.remaining()).also(buffer::get)
        return cipher.decrypt(EncryptedCredential(vector, ciphertext)).toString(Charsets.UTF_8)
    }

    override fun write(value: String) {
        val encrypted = cipher.encrypt(value.toByteArray(Charsets.UTF_8))
        val buffer = ByteBuffer.allocate(
            Integer.BYTES + encrypted.initializationVector.size + encrypted.ciphertext.size
        )
        buffer.putInt(encrypted.initializationVector.size)
        buffer.put(encrypted.initializationVector)
        buffer.put(encrypted.ciphertext)
        file.parentFile?.mkdirs()
        val temporaryFile = File.createTempFile("$FILE_NAME.", ".tmp", file.parentFile)
        try {
            temporaryFile.writeBytes(buffer.array())
            fileReplacer.replace(temporaryFile, file)
        } finally {
            temporaryFile.delete()
        }
    }

    override fun clear() {
        if (file.exists() && !file.delete()) file.writeBytes(byteArrayOf())
    }

    private companion object {
        const val FILE_NAME = "session_credentials.enc"
    }
}

class AndroidKeystoreCredentialCipher : CredentialCipher {
    private val key: SecretKey
        get() {
            val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER).apply { load(null) }
            return (keyStore.getKey(KEY_ALIAS, null) as? SecretKey) ?: generateKey()
        }

    override fun encrypt(plaintext: ByteArray): EncryptedCredential {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        return EncryptedCredential(cipher.iv, cipher.doFinal(plaintext))
    }

    override fun decrypt(value: EncryptedCredential): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(
            Cipher.DECRYPT_MODE,
            key,
            GCMParameterSpec(AUTHENTICATION_TAG_BITS, value.initializationVector)
        )
        return cipher.doFinal(value.ciphertext)
    }

    private fun generateKey(): SecretKey {
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE_PROVIDER)
        generator.init(
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        return generator.generateKey()
    }

    private companion object {
        const val KEYSTORE_PROVIDER = "AndroidKeyStore"
        const val KEY_ALIAS = "medtracker_session_credentials"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val AUTHENTICATION_TAG_BITS = 128
    }
}
