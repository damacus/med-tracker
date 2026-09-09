package io.damacus.medtracker.data.model

data class PersonDto(
    val id: Long? = null,
    val portableId: String? = null,
    val name: String,
    val email: String? = null,
    val dateOfBirth: String? = null,
    val personType: String? = null,
    val age: Int? = null,
    val hasCapacity: Boolean? = null
)

data class MedicationDto(
    val id: Long? = null,
    val portableId: String? = null,
    val name: String,
    val displayName: String? = null,
    val category: String? = null,
    val description: String? = null,
    val doseAmount: Double? = null,
    val doseUnit: String? = null,
    val currentSupply: Double? = null,
    val reorderThreshold: Double? = null,
    val reorderStatus: String? = null,
    val lowStock: Boolean? = null,
    val outOfStock: Boolean? = null
)

data class ScheduleDto(
    val id: Long? = null,
    val portableId: String? = null,
    val personId: Long? = null,
    val personPortableId: String? = null,
    val medicationId: Long? = null,
    val medicationPortableId: String? = null,
    val doseAmount: Double? = null,
    val doseUnit: String? = null,
    val frequency: String? = null,
    val doseCycle: String? = null,
    val startDate: String? = null,
    val endDate: String? = null,
    val active: Boolean = true,
    val paused: Boolean = false,
    val notes: String? = null,
    val maxDailyDoses: Int? = null,
    val minHoursBetweenDoses: Double? = null
)

data class MedicationTakeDto(
    val id: Long? = null,
    val portableId: String? = null,
    val clientUuid: String? = null,
    val scheduleId: Long? = null,
    val personMedicationId: Long? = null,
    val personId: Long? = null,
    val medicationId: Long? = null,
    val doseAmount: Double? = null,
    val doseUnit: String? = null,
    val takenAt: String? = null
)

data class RecordDosePayload(
    val clientUuid: String,
    val sourceType: String, // "schedule" or "person_medication"
    val sourceId: String,
    val takenAt: String,
    val doseAmount: Double? = null,
    val doseUnit: String? = null
)

data class LocationDto(
    val id: Long? = null,
    val name: String,
    val description: String? = null,
    val personIds: List<Long> = emptyList()
)

data class HouseholdInvitationDto(
    val id: Long? = null,
    val email: String,
    val role: String,
    val status: String? = null,
    val expiresAt: String? = null
)

data class CapabilitiesDto(
    val apiVersion: String,
    val format: String
)

data class AiSuggestionDto(
    val id: Long? = null,
    val title: String,
    val text: String,
    val doseCycle: String? = null
)

data class MedicationLookupResultDto(
    val id: Long? = null,
    val name: String,
    val category: String? = null,
    val description: String? = null
)

data class HealthEventDto(
    val id: Long? = null,
    val eventKind: String,
    val severity: String? = null,
    val title: String,
    val notes: String? = null,
    val occurredAt: String? = null
)

data class HouseholdAdminSettingsDto(
    val subscriptionPlan: String,
    val operational: Boolean = true
)

data class RecordNotTakenPayload(
    val clientUuid: String,
    val sourceType: String,
    val sourceId: String,
    val notTakenAt: String,
    val reason: String
)

data class RecordStockRemovalPayload(
    val medicationId: Long,
    val quantity: Double,
    val reason: String,
    val removedAt: String
)

data class CreateMedicationPayload(
    val name: String,
    val category: String? = null,
    val doseAmount: Double? = null,
    val doseUnit: String? = null,
    val currentSupply: Double? = null,
    val reorderThreshold: Double? = null
)

data class CreateSchedulePayload(
    val personId: Long,
    val medicationId: Long,
    val doseAmount: Double,
    val doseUnit: String,
    val frequency: String,
    val doseCycle: String? = null,
    val startDate: String
)

data class DashboardScheduleItem(
    val schedule: ScheduleDto,
    val person: PersonDto?,
    val medication: MedicationDto?,
    val lastTakenAt: String? = null,
    val isDueNow: Boolean = true,
    val isTaking: Boolean = false
)

data class DashboardData(
    val people: List<PersonDto> = emptyList(),
    val medications: List<MedicationDto> = emptyList(),
    val schedules: List<ScheduleDto> = emptyList(),
    val recentTakes: List<MedicationTakeDto> = emptyList(),
    val selectedPersonId: Long? = null, // null means "All family"
    val capabilities: CapabilitiesDto? = null,
    val aiSuggestions: List<AiSuggestionDto> = emptyList()
) {
    val selectedPerson: PersonDto?
        get() = if (selectedPersonId == null) null else people.find { it.id == selectedPersonId }

    val activeSchedules: List<ScheduleDto>
        get() = schedules.filter { sched ->
            sched.active && !sched.paused && (selectedPersonId == null || sched.personId == selectedPersonId)
        }

    val displayScheduleItems: List<DashboardScheduleItem>
        get() {
            val peopleMap = people.associateBy { it.id }
            val medMap = medications.associateBy { it.id }
            return activeSchedules.map { schedule ->
                val med = schedule.medicationId?.let { medMap[it] }
                val person = schedule.personId?.let { peopleMap[it] }
                val lastTake = recentTakes
                    .filter { it.scheduleId == schedule.id }
                    .maxByOrNull { it.takenAt ?: "" }

                DashboardScheduleItem(
                    schedule = schedule,
                    person = person,
                    medication = med,
                    lastTakenAt = lastTake?.takenAt
                )
            }
        }

    val lowStockMedications: List<MedicationDto>
        get() = medications.filter { it.lowStock == true || it.outOfStock == true }
}
