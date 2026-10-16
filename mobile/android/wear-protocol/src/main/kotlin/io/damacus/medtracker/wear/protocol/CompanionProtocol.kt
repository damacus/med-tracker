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

data class PersonData(
    val id: String,
    val name: String,
    val type: Int
)

data class MedicationData(
    val id: String,
    val personId: String,
    val name: String,
    val dose: String,
    val status: String,
    val dueTime: String? = null
)

data class CompanionSchedule(
    val publishedAt: Long,
    val people: List<PersonData>,
    val medications: List<MedicationData>
)

data class TakenDoseRecord(
    val eventId: String,
    val personId: String,
    val medicationId: String,
    val takenAt: Long,
    val dosage: String
)

object ScheduleProtocol {
    const val PATH = "/medtracker/companion/schedule"
    private const val MAX_BYTES = 32768

    fun encode(schedule: CompanionSchedule): ByteArray {
        require(schedule.publishedAt >= 0)
        return buildJsonObject {
            put("publishedAt", schedule.publishedAt)
            put("people", buildJsonArray {
                schedule.people.forEach { person ->
                    add(buildJsonObject {
                        put("id", person.id)
                        put("name", person.name)
                        put("type", person.type)
                    })
                }
            })
            put("medications", buildJsonArray {
                schedule.medications.forEach { med ->
                    add(buildJsonObject {
                        put("id", med.id)
                        put("personId", med.personId)
                        put("name", med.name)
                        put("dose", med.dose)
                        put("status", med.status)
                        med.dueTime?.let { put("dueTime", it) }
                    })
                }
            })
        }.toString().toByteArray(Charsets.UTF_8).also { require(it.size <= MAX_BYTES) }
    }

    fun decode(bytes: ByteArray): CompanionSchedule? {
        if (bytes.size > MAX_BYTES) return null
        return runCatching {
            val root = Json.parseToJsonElement(bytes.decodeToString(throwOnInvalidSequence = true)).jsonObject
            val publishedAt = root.getValue("publishedAt").jsonPrimitive.long
            val people = root.getValue("people").jsonArray.map { item ->
                val obj = item.jsonObject
                PersonData(
                    id = obj.getValue("id").jsonPrimitive.content,
                    name = obj.getValue("name").jsonPrimitive.content,
                    type = obj.getValue("type").jsonPrimitive.int
                )
            }
            val medications = root.getValue("medications").jsonArray.map { item ->
                val obj = item.jsonObject
                MedicationData(
                    id = obj.getValue("id").jsonPrimitive.content,
                    personId = obj.getValue("personId").jsonPrimitive.content,
                    name = obj.getValue("name").jsonPrimitive.content,
                    dose = obj.getValue("dose").jsonPrimitive.content,
                    status = obj.getValue("status").jsonPrimitive.content,
                    dueTime = obj["dueTime"]?.jsonPrimitive?.contentOrNull
                )
            }
            CompanionSchedule(publishedAt, people, medications)
        }.getOrNull()
    }
}

object DoseProtocol {
    const val PATH_PREFIX = "/medtracker/doses/taken/"
    private const val MAX_BYTES = 4096

    fun encode(record: TakenDoseRecord): ByteArray {
        require(record.eventId.isNotBlank() && record.takenAt >= 0)
        return buildJsonObject {
            put("eventId", record.eventId)
            put("personId", record.personId)
            put("medicationId", record.medicationId)
            put("takenAt", record.takenAt)
            put("dosage", record.dosage)
        }.toString().toByteArray(Charsets.UTF_8).also { require(it.size <= MAX_BYTES) }
    }

    fun decode(bytes: ByteArray): TakenDoseRecord? {
        if (bytes.size > MAX_BYTES) return null
        return runCatching {
            val root = Json.parseToJsonElement(bytes.decodeToString(throwOnInvalidSequence = true)).jsonObject
            TakenDoseRecord(
                eventId = root.getValue("eventId").jsonPrimitive.content,
                personId = root.getValue("personId").jsonPrimitive.content,
                medicationId = root.getValue("medicationId").jsonPrimitive.content,
                takenAt = root.getValue("takenAt").jsonPrimitive.long,
                dosage = root.getValue("dosage").jsonPrimitive.content
            )
        }.getOrNull()
    }
}
