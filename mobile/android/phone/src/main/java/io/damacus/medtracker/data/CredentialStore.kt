package io.damacus.medtracker.data

interface CredentialStore {
    fun read(): String?
    fun write(value: String)
    fun clear()
}
