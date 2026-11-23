package io.damacus.medtracker.data.api

import com.squareup.moshi.JsonDataException
import io.damacus.medtracker.data.model.AiSuggestionDto
import io.damacus.medtracker.data.model.CapabilitiesDto
import io.damacus.medtracker.data.model.CreateMedicationPayload
import io.damacus.medtracker.data.model.CreateSchedulePayload
import io.damacus.medtracker.data.model.HealthEventDto
import io.damacus.medtracker.data.model.HouseholdAdminSettingsDto
import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.HouseholdChoice
import io.damacus.medtracker.data.model.HouseholdSelection
import io.damacus.medtracker.data.model.HouseholdInvitationDto
import io.damacus.medtracker.data.model.LocationDto
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.MedicationLookupResultDto
import io.damacus.medtracker.data.model.MedicationTakeDto
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RecordStockRemovalPayload
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import io.medtracker.client.apis.AuditLogsApi
import io.medtracker.client.apis.AuthenticationApi
import io.medtracker.client.apis.CapabilitiesApi
import io.medtracker.client.apis.HealthEventsApi
import io.medtracker.client.apis.HouseholdApi
import io.medtracker.client.apis.HouseholdSettingsApi
import io.medtracker.client.apis.InvitationsApi
import io.medtracker.client.apis.LocationsApi
import io.medtracker.client.apis.MedicationLookupApi
import io.medtracker.client.apis.MedicationSuggestionsApi
import io.medtracker.client.apis.MedicationTakesApi
import io.medtracker.client.apis.MedicationsApi
import io.medtracker.client.apis.NotificationPreferencesApi
import io.medtracker.client.apis.PeopleApi
import io.medtracker.client.apis.PortabilityApi
import io.medtracker.client.apis.SchedulesApi
import io.medtracker.client.infrastructure.ClientException
import io.medtracker.client.infrastructure.ApiResponse
import io.medtracker.client.infrastructure.ClientError
import io.medtracker.client.infrastructure.Redirection
import io.medtracker.client.infrastructure.ServerException
import io.medtracker.client.infrastructure.ServerError
import io.medtracker.client.infrastructure.Serializer
import io.medtracker.client.infrastructure.Success
import io.medtracker.client.models.AiMedicationSuggestion
import io.medtracker.client.models.Capabilities
import io.medtracker.client.models.DoseNotTakenRequest
import io.medtracker.client.models.DoseNotTakenRequestDoseOccurrence
import io.medtracker.client.models.DoseOccurrence
import io.medtracker.client.models.DoseReopenRequest
import io.medtracker.client.models.DoseReopenRequestDoseOccurrence
import io.medtracker.client.models.ErrorEnvelope
import io.medtracker.client.models.HealthEvent
import io.medtracker.client.models.HealthEventCreateRequest
import io.medtracker.client.models.HealthEventCreateRequestHealthEvent
import io.medtracker.client.models.HouseholdAdminSettings
import io.medtracker.client.models.HouseholdAdminSettingsAttributes
import io.medtracker.client.models.HouseholdAdminSettingsUpdateRequest
import io.medtracker.client.models.HouseholdInvitation
import io.medtracker.client.models.HouseholdInvitationAttributes
import io.medtracker.client.models.HouseholdInvitationCreateRequest
import io.medtracker.client.models.Location
import io.medtracker.client.models.LocationCreateRequest
import io.medtracker.client.models.LocationCreateRequestLocation
import io.medtracker.client.models.Me
import io.medtracker.client.models.Medication
import io.medtracker.client.models.MedicationCreateAttributes
import io.medtracker.client.models.MedicationCreateRequest
import io.medtracker.client.models.MedicationInventoryAdjustmentRequest
import io.medtracker.client.models.MedicationInventoryAdjustmentRequestAdjustment
import io.medtracker.client.models.MedicationLookupResult
import io.medtracker.client.models.MedicationTake
import io.medtracker.client.models.MedicationTakeCreateRequest
import io.medtracker.client.models.MedicationTakeCreateRequestMedicationTake
import io.medtracker.client.models.NotificationPreference
import io.medtracker.client.models.NotificationPreferenceAttributes
import io.medtracker.client.models.NotificationPreferenceUpdateRequest
import io.medtracker.client.models.PaginationMeta
import io.medtracker.client.models.Person
import io.medtracker.client.models.PersonCreateRequest
import io.medtracker.client.models.PersonCreateRequestPerson
import io.medtracker.client.models.RateLimitError
import io.medtracker.client.models.RateLimitErrorEnvelope
import io.medtracker.client.models.Schedule
import io.medtracker.client.models.ScheduleCreateRequest
import io.medtracker.client.models.ScheduleCreateRequestSchedule
import io.medtracker.client.models.SecurityAuditEvent
import io.medtracker.client.models.StockRemovalReason
import io.medtracker.client.models.StockRemovalRequest
import io.medtracker.client.models.StockRemovalRequestStockRemoval
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Call
import java.io.IOException
import java.time.LocalDate
import java.time.OffsetDateTime
import java.time.format.DateTimeParseException
import java.util.UUID

data class DoseOccurrenceDto(
    val key: String,
    val scheduleId: Long,
    val outcome: String,
    val due: Boolean,
    val etag: String? = null,
    val reason: String? = null,
    val note: String? = null
)

data class AuditLogDto(
    val id: Long,
    val eventType: String,
    val actorAccountId: Long? = null,
    val actorMembershipId: Long? = null,
    val requestId: String? = null,
    val createdAt: String
)

data class PortableBackupExportDto(
    val filename: String,
    val contentType: String,
    val bytes: ByteArray
)

data class NotificationPreferencesDto(
    val enabled: Boolean = true,
    val doseDueEnabled: Boolean = true,
    val missedDoseEnabled: Boolean = true,
    val lowStockEnabled: Boolean = true,
    val privateTextEnabled: Boolean = false,
    val morningTime: String? = null,
    val afternoonTime: String? = null,
    val eveningTime: String? = null,
    val nightTime: String? = null
)

data class CreatePersonPayload(
    val name: String,
    val dateOfBirth: String,
    val personType: String = "minor",
    val initialGuardianDeclaration: Boolean = true,
    val email: String? = null
)

sealed class ApiResult<out T> {
    data class Success<T>(val data: T) : ApiResult<T>()
    data class Error(
        val code: String?,
        val message: String,
        val statusCode: Int? = null,
        val requestId: String? = null,
        val traceId: String? = null,
        val errors: Map<String, List<String>>? = null
    ) : ApiResult<Nothing>()
    data class NetworkError(val cause: Throwable) : ApiResult<Nothing>()
}

interface MedTrackerApi {
    suspend fun getHouseholds(baseUrl: String, accessToken: String): ApiResult<List<HouseholdChoice>> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getMe(baseUrl: String, accessToken: String, householdId: Long): ApiResult<UserDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getCapabilities(baseUrl: String): ApiResult<CapabilitiesDto> = ApiResult.Success(CapabilitiesDto("v1", "medtracker.api.capabilities.v1"))
    suspend fun logout(baseUrl: String, accessToken: String): ApiResult<Unit>
    suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<PersonDto>>
    suspend fun createPerson(baseUrl: String, accessToken: String, householdId: Long, request: CreatePersonPayload): ApiResult<PersonDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationDto>>
    suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<ScheduleDto>>
    suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationTakeDto>>
    suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload): ApiResult<MedicationTakeDto>
    suspend fun getScheduleDoseOccurrences(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long, startDate: String, endDate: String): ApiResult<List<DoseOccurrenceDto>> = ApiResult.Success(emptyList())
    suspend fun recordScheduleDoseNotTaken(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long, occurrenceKey: String, reason: String, note: String? = null): ApiResult<DoseOccurrenceDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun reopenScheduleDoseOccurrence(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long, occurrenceKey: String, etag: String): ApiResult<DoseOccurrenceDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<LocationDto>>
    suspend fun createLocation(baseUrl: String, accessToken: String, householdId: Long, name: String, description: String? = null): ApiResult<LocationDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HouseholdInvitationDto>>
    suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String): ApiResult<HouseholdInvitationDto>
    suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload): ApiResult<MedicationDto>
    suspend fun adjustInventory(baseUrl: String, accessToken: String, householdId: Long, medicationId: Long, newQuantity: Double, reason: String? = null): ApiResult<MedicationDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun markMedicationOrdered(baseUrl: String, accessToken: String, householdId: Long, medicationId: Long): ApiResult<MedicationDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun markMedicationReceived(baseUrl: String, accessToken: String, householdId: Long, medicationId: Long): ApiResult<MedicationDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload): ApiResult<MedicationDto>
    suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload): ApiResult<ScheduleDto>
    suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto>
    suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto>
    suspend fun getAiMedicationSuggestions(baseUrl: String, accessToken: String, householdId: Long): ApiResult<AiSuggestionDto?> = ApiResult.Success(null)
    suspend fun lookupMedication(baseUrl: String, accessToken: String, householdId: Long, query: String): ApiResult<List<MedicationLookupResultDto>> = ApiResult.Success(emptyList())
    suspend fun getHealthEvents(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HealthEventDto>> = ApiResult.Success(emptyList())
    suspend fun createHealthEvent(baseUrl: String, accessToken: String, householdId: Long, personId: Long, title: String, notes: String, eventKind: String = "suspected_side_effect", startedOn: String? = null): ApiResult<HealthEventDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getAdminSettings(baseUrl: String, accessToken: String, householdId: Long): ApiResult<HouseholdAdminSettingsDto> = ApiResult.Success(HouseholdAdminSettingsDto("free"))
    suspend fun updateAdminSettings(baseUrl: String, accessToken: String, householdId: Long, name: String? = null, timezone: String? = null, aiSuggestionsEnabled: Boolean? = null): ApiResult<HouseholdAdminSettingsDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getAuditLogs(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<AuditLogDto>> = ApiResult.Success(emptyList())
    suspend fun exportPortableBackup(baseUrl: String, accessToken: String, householdId: Long): ApiResult<PortableBackupExportDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getNotificationPreferences(baseUrl: String, accessToken: String, householdId: Long): ApiResult<NotificationPreferencesDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun updateNotificationPreferences(baseUrl: String, accessToken: String, householdId: Long, preferences: NotificationPreferencesDto): ApiResult<NotificationPreferencesDto> = ApiResult.Error("not_implemented", "Not implemented")
}

private val configureNullSafeEnums: Unit = run {
    io.medtracker.client.infrastructure.Serializer.moshiBuilder.add(object : com.squareup.moshi.JsonAdapter.Factory {
        override fun create(type: java.lang.reflect.Type, annotations: Set<Annotation>, moshi: com.squareup.moshi.Moshi): com.squareup.moshi.JsonAdapter<*>? {
            val rawType = com.squareup.moshi.Types.getRawType(type)
            if (rawType.isEnum) {
                return moshi.nextAdapter<Any>(this, type, annotations).nullSafe()
            }
            return null
        }
    })
}

class GeneratedMedTrackerApi(
    internal val callFactory: Call.Factory = HttpLoggingPolicy.client()
) : MedTrackerApi {
    init {
        configureNullSafeEnums
    }

    private val unauthenticatedCalls = RequestAuthCallFactory(callFactory)

    override suspend fun getHouseholds(baseUrl: String, accessToken: String) = authenticated(accessToken) { requestCalls ->
        AuthenticationApi(apiBaseUrl(baseUrl), requestCalls).listHouseholds().data.map {
            HouseholdChoice(it.id.toLong(), it.name, it.role.value)
        }
    }

    override suspend fun getMe(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        HouseholdApi(apiBaseUrl(baseUrl), requestCalls).getCurrentProfile(householdId.toInt()).data.toDomain()
    }

    override suspend fun getCapabilities(baseUrl: String) = generated {
        CapabilitiesApi(apiBaseUrl(baseUrl), unauthenticatedCalls).getCapabilities().data.toDomain()
    }

    override suspend fun logout(baseUrl: String, accessToken: String): ApiResult<Unit> =
        authenticated(accessToken) { requestCalls ->
            AuthenticationApi(apiBaseUrl(baseUrl), requestCalls).logoutSession()
        }.let { result ->
            if (result is ApiResult.Error && result.statusCode == 401) ApiResult.Success(Unit) else result
        }

    override suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        val peopleApi = PeopleApi(apiBaseUrl(baseUrl), requestCalls)
        fetchAllPages(
            fetchPage = { page, perPage ->
                val res = peopleApi.listPeople(householdId.toInt(), page = page, perPage = perPage)
                res.data to res.meta
            },
            transform = Person::toDomain
        )
    }

    override suspend fun createPerson(baseUrl: String, accessToken: String, householdId: Long, request: CreatePersonPayload): ApiResult<PersonDto> {
        val dob = try {
            LocalDate.parse(request.dateOfBirth.trim())
        } catch (_: DateTimeParseException) {
            return ApiResult.Error("invalid_date", "Invalid date of birth. Use YYYY-MM-DD.")
        }
        val personType = when (request.personType.trim().lowercase()) {
            "dependent_adult" -> PersonCreateRequestPerson.PersonType.dependent_adult
            else -> PersonCreateRequestPerson.PersonType.minor
        }
        return authenticated(accessToken) { requestCalls ->
            PeopleApi(apiBaseUrl(baseUrl), requestCalls).createPerson(
                householdId.toInt(),
                PersonCreateRequest(
                    PersonCreateRequestPerson(
                        name = request.name.trim(),
                        dateOfBirth = dob,
                        personType = personType,
                        initialGuardianDeclaration = request.initialGuardianDeclaration,
                        email = request.email?.trim()?.ifBlank { null }
                    )
                )
            ).data.toDomain()
        }
    }

    override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        val medsApi = MedicationsApi(apiBaseUrl(baseUrl), requestCalls)
        fetchAllPages(
            fetchPage = { page, perPage ->
                val res = medsApi.listMedications(householdId.toInt(), page = page, perPage = perPage)
                res.data to res.meta
            },
            transform = Medication::toDomain
        )
    }

    override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        val schedulesApi = SchedulesApi(apiBaseUrl(baseUrl), requestCalls)
        fetchAllPages(
            fetchPage = { page, perPage ->
                val res = schedulesApi.listSchedules(householdId.toInt(), page = page, perPage = perPage)
                res.data to res.meta
            },
            transform = Schedule::toDomain
        )
    }

    override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        val takesApi = MedicationTakesApi(apiBaseUrl(baseUrl), requestCalls)
        fetchAllPages(
            fetchPage = { page, perPage ->
                val res = takesApi.listMedicationTakes(householdId.toInt(), page = page, perPage = perPage)
                res.data to res.meta
            },
            transform = MedicationTake::toDomain
        )
    }

    override suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload) = authenticated(accessToken) { requestCalls ->
        MedicationTakesApi(apiBaseUrl(baseUrl), requestCalls).createMedicationTake(householdId.toInt(), MedicationTakeCreateRequest(
            MedicationTakeCreateRequestMedicationTake(
                when (request.sourceType) {
                    "schedule" -> MedicationTakeCreateRequestMedicationTake.SourceType.schedule
                    "person_medication" -> MedicationTakeCreateRequestMedicationTake.SourceType.person_medication
                    else -> MedicationTakeCreateRequestMedicationTake.SourceType.unknown_default_open_api
                }, request.sourceId, OffsetDateTime.parse(request.takenAt), UUID.fromString(request.clientUuid),
                request.doseAmount?.toString(), request.doseUnit
            )
        )).data.toDomain()
    }

    override suspend fun getScheduleDoseOccurrences(
        baseUrl: String,
        accessToken: String,
        householdId: Long,
        scheduleId: Long,
        startDate: String,
        endDate: String
    ): ApiResult<List<DoseOccurrenceDto>> {
        val parsedStart = try {
            LocalDate.parse(startDate)
        } catch (_: DateTimeParseException) {
            return ApiResult.Error("invalid_date", "Invalid start date. Use YYYY-MM-DD.")
        }
        val parsedEnd = try {
            LocalDate.parse(endDate)
        } catch (_: DateTimeParseException) {
            return ApiResult.Error("invalid_date", "Invalid end date. Use YYYY-MM-DD.")
        }
        return authenticated(accessToken) { requestCalls ->
            HouseholdApi(apiBaseUrl(baseUrl), requestCalls)
                .listScheduleDoseOccurrences(householdId.toInt(), scheduleId.toString(), parsedStart, parsedEnd)
                .data
                .map(DoseOccurrence::toDomain)
        }
    }

    override suspend fun recordScheduleDoseNotTaken(
        baseUrl: String,
        accessToken: String,
        householdId: Long,
        scheduleId: Long,
        occurrenceKey: String,
        reason: String,
        note: String?
    ) = authenticated(accessToken) { requestCalls ->
        val reasonEnum = DoseNotTakenRequestDoseOccurrence.Reason.entries.firstOrNull {
            it.value.equals(reason.trim(), ignoreCase = true)
        } ?: DoseNotTakenRequestDoseOccurrence.Reason.other
        HouseholdApi(apiBaseUrl(baseUrl), requestCalls)
            .recordScheduleDoseNotTaken(
                householdId.toInt(),
                scheduleId.toString(),
                DoseNotTakenRequest(
                    DoseNotTakenRequestDoseOccurrence(
                        key = occurrenceKey,
                        reason = reasonEnum,
                        note = note?.trim()?.ifBlank { null }
                    )
                ),
                idempotencyKey = UUID.randomUUID().toString()
            )
            .data
            .toDomain()
    }

    override suspend fun reopenScheduleDoseOccurrence(
        baseUrl: String,
        accessToken: String,
        householdId: Long,
        scheduleId: Long,
        occurrenceKey: String,
        etag: String
    ) = authenticated(accessToken) { requestCalls ->
        HouseholdApi(apiBaseUrl(baseUrl), requestCalls)
            .reopenScheduleDoseOccurrence(
                householdId.toInt(),
                scheduleId.toString(),
                ifMatch = etag,
                DoseReopenRequest(DoseReopenRequestDoseOccurrence(key = occurrenceKey)),
                idempotencyKey = UUID.randomUUID().toString()
            )
            .data
            .toDomain()
    }

    override suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        val locationsApi = LocationsApi(apiBaseUrl(baseUrl), requestCalls)
        fetchAllPages(
            fetchPage = { page, perPage ->
                val res = locationsApi.listLocations(householdId.toInt(), page = page, perPage = perPage)
                res.data to res.meta
            },
            transform = Location::toDomain
        )
    }

    override suspend fun createLocation(baseUrl: String, accessToken: String, householdId: Long, name: String, description: String?) = authenticated(accessToken) { requestCalls ->
        LocationsApi(apiBaseUrl(baseUrl), requestCalls).createLocation(
            householdId.toInt(),
            LocationCreateRequest(
                LocationCreateRequestLocation(
                    name = name.trim(),
                    description = description?.trim()?.ifBlank { null }
                )
            )
        ).data.toDomain()
    }

    override suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        InvitationsApi(apiBaseUrl(baseUrl), requestCalls).listInvitations(householdId.toInt()).data.map(HouseholdInvitation::toDomain)
    }

    override suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String) = authenticated(accessToken) { requestCalls ->
        InvitationsApi(apiBaseUrl(baseUrl), requestCalls).createInvitation(
            householdId.toInt(),
            HouseholdInvitationCreateRequest(HouseholdInvitationAttributes(
                email = email,
                membershipRole = if (role == "administrator") HouseholdInvitationAttributes.MembershipRole.administrator else HouseholdInvitationAttributes.MembershipRole.member
            ))
        ).data.toDomain()
    }

    override suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload) = authenticated(accessToken) { requestCalls ->
        val parsedDoseUnit = request.doseUnit?.trim()?.takeIf { it.isNotBlank() }?.let { raw ->
            MedicationCreateAttributes.DoseUnit.entries.firstOrNull {
                it != MedicationCreateAttributes.DoseUnit.unknown_default_open_api &&
                    it.value.equals(raw, ignoreCase = true)
            }
        }
        MedicationsApi(apiBaseUrl(baseUrl), requestCalls).createMedication(
            householdId.toInt(),
            MedicationCreateRequest(MedicationCreateAttributes(
                name = request.name,
                reorderThreshold = request.reorderThreshold?.toString() ?: "0",
                locationId = locationId,
                category = request.category,
                doseAmount = request.doseAmount?.toString(),
                doseUnit = parsedDoseUnit,
                currentSupply = request.currentSupply?.toString()
            ))
        ).data.toDomain()
    }

    override suspend fun adjustInventory(baseUrl: String, accessToken: String, householdId: Long, medicationId: Long, newQuantity: Double, reason: String?) = authenticated(accessToken) { requestCalls ->
        MedicationsApi(apiBaseUrl(baseUrl), requestCalls).adjustMedicationInventory(
            householdId.toInt(),
            medicationId.toString(),
            MedicationInventoryAdjustmentRequest(
                MedicationInventoryAdjustmentRequestAdjustment(
                    newQuantity = String.format(java.util.Locale.US, "%.2f", newQuantity),
                    reason = reason?.trim()?.ifBlank { null }
                )
            )
        ).data.toDomain()
    }

    override suspend fun markMedicationOrdered(baseUrl: String, accessToken: String, householdId: Long, medicationId: Long) = authenticated(accessToken) { requestCalls ->
        MedicationsApi(apiBaseUrl(baseUrl), requestCalls).markMedicationAsOrdered(
            householdId.toInt(),
            medicationId.toString()
        ).data.toDomain()
    }

    override suspend fun markMedicationReceived(baseUrl: String, accessToken: String, householdId: Long, medicationId: Long) = authenticated(accessToken) { requestCalls ->
        MedicationsApi(apiBaseUrl(baseUrl), requestCalls).markMedicationAsReceived(
            householdId.toInt(),
            medicationId.toString()
        ).data.toDomain()
    }

    override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload) = authenticated(accessToken) { requestCalls ->
        val medicationsApi = MedicationsApi(apiBaseUrl(baseUrl), requestCalls)
        val removalReason = StockRemovalReason.entries.firstOrNull {
            it != StockRemovalReason.unknown_default_open_api &&
                it.value.equals(request.reason.trim(), ignoreCase = true)
        } ?: StockRemovalReason.other
        val submissionUuid = request.submissionId?.let { raw ->
            runCatching { UUID.fromString(raw) }.getOrNull()
        } ?: UUID.randomUUID()
        medicationsApi.createMedicationStockRemoval(
            householdId.toInt(),
            request.medicationId.toString(),
            StockRemovalRequest(
                StockRemovalRequestStockRemoval(
                    quantity = String.format(java.util.Locale.US, "%.2f", request.quantity),
                    reason = removalReason,
                    submissionId = submissionUuid,
                    note = request.reason
                )
            )
        )
        medicationsApi.getMedication(householdId.toInt(), request.medicationId.toString()).data.toDomain()
    }

    override suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload): ApiResult<ScheduleDto> {
        val startDate = try {
            LocalDate.parse(request.startDate.trim())
        } catch (_: DateTimeParseException) {
            return ApiResult.Error("invalid_date", "Invalid start date. Use YYYY-MM-DD.")
        }
        return authenticated(accessToken) { requestCalls ->
            SchedulesApi(apiBaseUrl(baseUrl), requestCalls).createSchedule(
                householdId.toInt(),
                ScheduleCreateRequest(
                    ScheduleCreateRequestSchedule(
                        personId = request.personId.toString(),
                        medicationId = request.medicationId.toString(),
                        doseAmount = request.doseAmount.toString(),
                        doseUnit = request.doseUnit,
                        startDate = startDate,
                        endDate = startDate.plusYears(1),
                        frequency = request.frequency
                    )
                )
            ).data.toDomain()
        }
    }

    override suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = authenticated(accessToken) { requestCalls ->
        SchedulesApi(apiBaseUrl(baseUrl), requestCalls).pauseSchedule(
            householdId.toInt(),
            scheduleId.toString()
        ).data.toDomain()
    }

    override suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long) = authenticated(accessToken) { requestCalls ->
        SchedulesApi(apiBaseUrl(baseUrl), requestCalls).resumeSchedule(
            householdId.toInt(),
            scheduleId.toString()
        ).data.toDomain()
    }

    override suspend fun getAiMedicationSuggestions(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        MedicationSuggestionsApi(apiBaseUrl(baseUrl), requestCalls).generateAiMedicationSuggestions(householdId.toInt()).data.toDomain()
    }

    override suspend fun lookupMedication(baseUrl: String, accessToken: String, householdId: Long, query: String) = authenticated(accessToken) { requestCalls ->
        MedicationLookupApi(apiBaseUrl(baseUrl), requestCalls).searchMedicationLookup(householdId.toInt(), q = query).results.map(MedicationLookupResult::toDomain)
    }

    override suspend fun getHealthEvents(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        val healthEventsApi = HealthEventsApi(apiBaseUrl(baseUrl), requestCalls)
        fetchAllPages(
            fetchPage = { page, perPage ->
                val res = healthEventsApi.listHealthEvents(householdId.toInt(), page = page, perPage = perPage)
                res.data to res.meta
            },
            transform = HealthEvent::toDomain
        )
    }

    override suspend fun createHealthEvent(
        baseUrl: String,
        accessToken: String,
        householdId: Long,
        personId: Long,
        title: String,
        notes: String,
        eventKind: String,
        startedOn: String?
    ): ApiResult<HealthEventDto> {
        val parsedStartedOn = if (startedOn.isNullOrBlank()) {
            LocalDate.now()
        } else {
            try {
                LocalDate.parse(startedOn.trim())
            } catch (_: DateTimeParseException) {
                return ApiResult.Error("invalid_date", "Invalid start date. Use YYYY-MM-DD.")
            }
        }
        val kindEnum = when (eventKind.trim().lowercase()) {
            "illness" -> HealthEventCreateRequestHealthEvent.EventKind.illness
            else -> HealthEventCreateRequestHealthEvent.EventKind.suspected_side_effect
        }
        return authenticated(accessToken) { requestCalls ->
            HealthEventsApi(apiBaseUrl(baseUrl), requestCalls).createHealthEvent(
                householdId.toInt(),
                HealthEventCreateRequest(
                    HealthEventCreateRequestHealthEvent(
                        personId = personId.toString(),
                        eventKind = kindEnum,
                        title = title.trim(),
                        startedOn = parsedStartedOn,
                        notes = notes.trim().ifBlank { null }
                    )
                )
            ).data.toDomain()
        }
    }

    override suspend fun getAdminSettings(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        HouseholdSettingsApi(apiBaseUrl(baseUrl), requestCalls).getHouseholdAdminSettings(householdId.toInt()).data.toDomain()
    }

    override suspend fun updateAdminSettings(
        baseUrl: String,
        accessToken: String,
        householdId: Long,
        name: String?,
        timezone: String?,
        aiSuggestionsEnabled: Boolean?
    ) = authenticated(accessToken) { requestCalls ->
        HouseholdSettingsApi(apiBaseUrl(baseUrl), requestCalls).updateHouseholdAdminSettings(
            householdId.toInt(),
            HouseholdAdminSettingsUpdateRequest(
                HouseholdAdminSettingsAttributes(
                    name = name?.trim()?.ifBlank { null },
                    timezone = timezone?.trim()?.ifBlank { null },
                    aiSuggestionsEnabled = aiSuggestionsEnabled
                )
            ),
            idempotencyKey = UUID.randomUUID().toString()
        ).data.toDomain()
    }

    override suspend fun getAuditLogs(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        AuditLogsApi(apiBaseUrl(baseUrl), requestCalls).listAuditLogs(householdId.toInt()).data.map(SecurityAuditEvent::toDomain)
    }

    override suspend fun exportPortableBackup(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        val data = PortabilityApi(apiBaseUrl(baseUrl), requestCalls).getDataExport(
            householdId.toInt(),
            PortabilityApi.ModeGetDataExport.backup_zip
        ).data
        PortableBackupExportDto(
            filename = data.filename ?: "medtracker-backup-$householdId.zip",
            contentType = data.contentType?.value ?: "application/zip",
            bytes = data.base64 ?: ByteArray(0)
        )
    }

    override suspend fun getNotificationPreferences(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        NotificationPreferencesApi(apiBaseUrl(baseUrl), requestCalls).getNotificationPreference(householdId.toInt()).data.toDomain()
    }

    override suspend fun updateNotificationPreferences(
        baseUrl: String,
        accessToken: String,
        householdId: Long,
        preferences: NotificationPreferencesDto
    ) = authenticated(accessToken) { requestCalls ->
        NotificationPreferencesApi(apiBaseUrl(baseUrl), requestCalls).updateNotificationPreference(
            householdId.toInt(),
            NotificationPreferenceUpdateRequest(
                NotificationPreferenceAttributes(
                    enabled = preferences.enabled,
                    doseDueEnabled = preferences.doseDueEnabled,
                    missedDoseEnabled = preferences.missedDoseEnabled,
                    lowStockEnabled = preferences.lowStockEnabled,
                    privateTextEnabled = preferences.privateTextEnabled,
                    morningTime = preferences.morningTime,
                    afternoonTime = preferences.afternoonTime,
                    eveningTime = preferences.eveningTime,
                    nightTime = preferences.nightTime
                )
            )
        ).data.toDomain()
    }

    private inline fun <T, R> fetchAllPages(
        perPage: Int = 100,
        maxPages: Int = 50,
        fetchPage: (page: Int, perPage: Int) -> Pair<List<T>, PaginationMeta>,
        transform: (T) -> R
    ): List<R> {
        val all = mutableListOf<R>()
        var page = 1
        while (page <= maxPages) {
            val (items, meta) = fetchPage(page, perPage)
            all.addAll(items.map(transform))
            if (items.isEmpty() || all.size >= meta.totalCount || items.size < meta.perPage) {
                break
            }
            page += 1
        }
        return all
    }

    internal suspend fun <T> generated(block: () -> T): ApiResult<T> = withContext(Dispatchers.IO) {
        try { ApiResult.Success(block()) } catch (error: ClientException) { error.toResult() }
        catch (error: ServerException) { error.toResult() } catch (error: IOException) { ApiResult.NetworkError(error) }
    }

    private suspend fun <T> authenticated(accessToken: String, block: (Call.Factory) -> T): ApiResult<T> = generated {
        block(RequestAuthCallFactory(callFactory, accessToken))
    }

    private fun ClientException.toResult() = errorResult(statusCode, (response as? ClientError<*>)?.body)
    private fun ServerException.toResult() = errorResult(statusCode, (response as? ServerError<*>)?.body)

    private fun errorResult(statusCode: Int, body: Any?): ApiResult.Error {
        val bodyString = body as? String
        val error = try {
            bodyString?.let { Serializer.moshi.adapter(ErrorEnvelope::class.java).fromJson(it)?.error }
        } catch (_: JsonDataException) {
            null
        } catch (_: IOException) {
            null
        }
        if (error != null && error.code.isNotBlank() && error.message.isNotBlank() && error.requestId.isNotBlank()) {
            return ApiResult.Error(
                code = error.code,
                message = error.message,
                statusCode = statusCode,
                requestId = error.requestId,
                traceId = error.traceId,
                errors = error.errors?.takeIf { it.isNotEmpty() }
            )
        }
        if (statusCode == 429) {
            val rateLimit = try {
                bodyString?.let { Serializer.moshi.adapter(RateLimitErrorEnvelope::class.java).fromJson(it)?.error }
            } catch (_: JsonDataException) {
                null
            } catch (_: IOException) {
                null
            }
            if (rateLimit != null && rateLimit.code != RateLimitError.Code.unknown_default_open_api && rateLimit.message.isNotBlank()) {
                return ApiResult.Error(
                    code = rateLimit.code.value,
                    message = rateLimit.message,
                    statusCode = statusCode,
                    requestId = null,
                    traceId = rateLimit.traceId
                )
            }
        }
        val flat = try {
            bodyString?.let { Serializer.moshi.adapter(Map::class.java).fromJson(it) }
        } catch (_: JsonDataException) {
            null
        } catch (_: IOException) {
            null
        }
        val isFlatError = flat?.get("error") is String ||
            (flat?.get("code") is String && flat["message"] is String)
        val nestedError = flat?.get("error") as? Map<*, *>
        val traceId = if (isFlatError) flat?.get("trace_id") as? String
            else nestedError?.get("trace_id") as? String
        return ApiResult.Error("http_$statusCode", "Request failed (HTTP $statusCode).", statusCode, traceId = traceId)
    }

    internal fun apiBaseUrl(baseUrl: String): String = "${baseUrl.trimEnd('/')}/api/v1"

}

private fun Me.toDomain() = UserDto(
    id = id.toLong(),
    emailAddress = emailAddress,
    name = person.name,
    role = membershipRole?.value
)
private fun DoseOccurrence.toDomain() = DoseOccurrenceDto(
    key = key,
    scheduleId = sourceId.toLong(),
    outcome = outcome.value,
    due = due,
    etag = etag,
    reason = reason?.value,
    note = note
)

private fun Person.toDomain() = PersonDto(id.toLong(), portableId.toString(), name, email, dateOfBirth?.toString(), personType.value, age, hasCapacity)
private fun Medication.toDomain() = MedicationDto(id.toLong(), portableId.toString(), name, displayName, category, description, doseAmount?.toDoubleOrNull(), doseUnit, currentSupply?.toDoubleOrNull(), reorderThreshold?.toDoubleOrNull(), reorderStatus?.value, lowStock, outOfStock)
private fun Schedule.toDomain() = ScheduleDto(id.toLong(), portableId.toString(), personId.toLong(), personPortableId.toString(), medicationId.toLong(), medicationPortableId.toString(), doseAmount.toDoubleOrNull(), doseUnit, frequency, doseCycle?.value, startDate.toString(), endDate.toString(), active, paused, notes, maxDailyDoses, minHoursBetweenDoses?.toDoubleOrNull())
private fun MedicationTake.toDomain() = MedicationTakeDto(id.toLong(), portableId.toString(), clientUuid?.toString(), scheduleId?.toLong(), personMedicationId?.toLong(), personId?.toLong(), medicationId?.toLong(), doseAmount?.toDoubleOrNull(), doseUnit, takenAt?.toString())
private fun Location.toDomain() = LocationDto(id = id.toLong(), name = name, description = description)
private fun HouseholdInvitation.toDomain(): HouseholdInvitationDto {
    val resolvedStatus = when {
        revokedAt != null -> "revoked"
        acceptedAt != null -> "accepted"
        expiresAt.isBefore(OffsetDateTime.now()) -> "expired"
        pending -> "pending"
        else -> "expired"
    }
    return HouseholdInvitationDto(
        id = id.toLong(),
        email = email,
        role = membershipRole.value,
        status = resolvedStatus,
        expiresAt = expiresAt.toString()
    )
}
private fun Capabilities.toDomain() = CapabilitiesDto(apiVersion = apiVersion.value, format = format.value)
private fun AiMedicationSuggestion.toDomain(): AiSuggestionDto {
    val dose = doses.firstOrNull()
    return AiSuggestionDto(
        title = medication.name ?: "Suggested Medication",
        text = medication.description ?: "AI Medication Suggestion",
        doseCycle = dose?.defaultDoseCycle?.value
    )
}
private fun MedicationLookupResult.toDomain() = MedicationLookupResultDto(id = null, name = name ?: display, category = category, description = description)
private fun HealthEvent.toDomain() = HealthEventDto(id = id.toLong(), eventKind = eventKind.value, severity = severity?.value, title = title, notes = notes, occurredAt = startedOn.toString())
private fun HouseholdAdminSettings.toDomain() = HouseholdAdminSettingsDto(
    subscriptionPlan = subscriptionPlan.value,
    name = name,
    timezone = timezone,
    aiSuggestionsEnabled = aiSuggestionsEnabled
)
private fun SecurityAuditEvent.toDomain() = AuditLogDto(
    id = id.toLong(),
    eventType = eventType,
    actorAccountId = actorAccountId?.toLong(),
    actorMembershipId = actorMembershipId?.toLong(),
    requestId = requestId,
    createdAt = createdAt.toString()
)
private fun NotificationPreference.toDomain() = NotificationPreferencesDto(
    enabled = enabled,
    doseDueEnabled = doseDueEnabled,
    missedDoseEnabled = missedDoseEnabled,
    lowStockEnabled = lowStockEnabled,
    privateTextEnabled = privateTextEnabled,
    morningTime = morningTime,
    afternoonTime = afternoonTime,
    eveningTime = eveningTime,
    nightTime = nightTime
)
