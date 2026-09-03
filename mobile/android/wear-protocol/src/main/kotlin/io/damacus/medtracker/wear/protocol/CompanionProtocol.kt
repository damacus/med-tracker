package io.damacus.medtracker.wear.protocol

import kotlinx.serialization.json.*

enum class SessionState(val wireValue: String) {
    SIGNED_IN("signed_in"), SIGNED_OUT("signed_out")
}

data class CompanionStatus(
    val phoneAppVersion: String,
    val sessionState: SessionState,
    val publishedAt: Long
)

sealed interface DecodedStatus {
    data class Supported(val status: CompanionStatus) : DecodedStatus
    data class Unsupported(val version: Int) : DecodedStatus
    data object Malformed : DecodedStatus
}

object CompanionProtocol {
    const val CAPABILITY = "medtracker_phone_companion_v1"
    const val STATUS_PATH = "/medtracker/companion/status"
    const val VERSION = 1
    private const val MAX_BYTES = 4096

    fun encode(status: CompanionStatus): ByteArray {
        require(status.phoneAppVersion.isNotBlank() && status.publishedAt >= 0)
        return buildJsonObject {
            put("protocolVersion", VERSION)
            put("phoneAppVersion", status.phoneAppVersion)
            put("sessionState", status.sessionState.wireValue)
            put("publishedAt", status.publishedAt)
        }.toString().toByteArray(Charsets.UTF_8).also { require(it.size <= MAX_BYTES) }
    }

    fun decode(bytes: ByteArray): DecodedStatus {
        if (bytes.size > MAX_BYTES) return DecodedStatus.Malformed
        return runCatching {
            val payload = Json.parseToJsonElement(bytes.decodeToString(throwOnInvalidSequence = true)).jsonObject
            val version = payload.getValue("protocolVersion").jsonPrimitive
            require(!version.isString)
            if (version.int != VERSION) return DecodedStatus.Unsupported(version.int)
            require(payload.keys == setOf("protocolVersion", "phoneAppVersion", "sessionState", "publishedAt"))
            val appVersion = payload.getValue("phoneAppVersion").jsonPrimitive
            val session = payload.getValue("sessionState").jsonPrimitive
            val time = payload.getValue("publishedAt").jsonPrimitive
            require(appVersion.isString && appVersion.content.isNotBlank())
            require(session.isString && !time.isString && time.long >= 0)
            DecodedStatus.Supported(CompanionStatus(
                appVersion.content,
                SessionState.entries.first { it.wireValue == session.content },
                time.long
            ))
        }.getOrDefault(DecodedStatus.Malformed)
    }
}
