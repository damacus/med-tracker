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
import io.medtracker.client.apis.PeopleApi
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
import io.medtracker.client.models.HouseholdInvitation
import io.medtracker.client.models.HouseholdInvitationAttributes
import io.medtracker.client.models.HouseholdInvitationCreateRequest
import io.medtracker.client.models.Location
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
import io.medtracker.client.models.Person
import io.medtracker.client.models.RateLimitError
import io.medtracker.client.models.RateLimitErrorEnvelope
import io.medtracker.client.models.Schedule
import io.medtracker.client.models.ScheduleCreateRequest
import io.medtracker.client.models.ScheduleCreateRequestSchedule
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
    suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationDto>>
    suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<ScheduleDto>>
    suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationTakeDto>>
    suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload): ApiResult<MedicationTakeDto>
    suspend fun getScheduleDoseOccurrences(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long, startDate: String, endDate: String): ApiResult<List<DoseOccurrenceDto>> = ApiResult.Success(emptyList())
    suspend fun recordScheduleDoseNotTaken(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long, occurrenceKey: String, reason: String, note: String? = null): ApiResult<DoseOccurrenceDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun reopenScheduleDoseOccurrence(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long, occurrenceKey: String, etag: String): ApiResult<DoseOccurrenceDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getLocations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<LocationDto>>
    suspend fun getInvitations(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HouseholdInvitationDto>>
    suspend fun createInvitation(baseUrl: String, accessToken: String, householdId: Long, email: String, role: String): ApiResult<HouseholdInvitationDto>
    suspend fun createMedication(baseUrl: String, accessToken: String, householdId: Long, locationId: Int, request: CreateMedicationPayload): ApiResult<MedicationDto>
    suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload): ApiResult<MedicationDto>
    suspend fun createSchedule(baseUrl: String, accessToken: String, householdId: Long, request: CreateSchedulePayload): ApiResult<ScheduleDto>
    suspend fun pauseSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto>
    suspend fun resumeSchedule(baseUrl: String, accessToken: String, householdId: Long, scheduleId: Long): ApiResult<ScheduleDto>
    suspend fun getAiMedicationSuggestions(baseUrl: String, accessToken: String, householdId: Long): ApiResult<AiSuggestionDto?> = ApiResult.Success(null)
    suspend fun lookupMedication(baseUrl: String, accessToken: String, householdId: Long, query: String): ApiResult<List<MedicationLookupResultDto>> = ApiResult.Success(emptyList())
    suspend fun getHealthEvents(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<HealthEventDto>> = ApiResult.Success(emptyList())
    suspend fun createHealthEvent(baseUrl: String, accessToken: String, householdId: Long, personId: Long, title: String, notes: String): ApiResult<HealthEventDto> = ApiResult.Error("not_implemented", "Not implemented")
    suspend fun getAdminSettings(baseUrl: String, accessToken: String, householdId: Long): ApiResult<HouseholdAdminSettingsDto> = ApiResult.Success(HouseholdAdminSettingsDto("free"))
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
        PeopleApi(apiBaseUrl(baseUrl), requestCalls).listPeople(householdId.toInt()).data.map(Person::toDomain)
    }

    override suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        MedicationsApi(apiBaseUrl(baseUrl), requestCalls).listMedications(householdId.toInt()).data.map(Medication::toDomain)
    }

    override suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        SchedulesApi(apiBaseUrl(baseUrl), requestCalls).listSchedules(householdId.toInt()).data.map(Schedule::toDomain)
    }

    override suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        MedicationTakesApi(apiBaseUrl(baseUrl), requestCalls).listMedicationTakes(householdId.toInt()).data.map(MedicationTake::toDomain)
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
        LocationsApi(apiBaseUrl(baseUrl), requestCalls).listLocations(householdId.toInt()).data.map(Location::toDomain)
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

    override suspend fun recordStockRemoval(baseUrl: String, accessToken: String, householdId: Long, request: RecordStockRemovalPayload) = authenticated(accessToken) { requestCalls ->
        val medicationsApi = MedicationsApi(apiBaseUrl(baseUrl), requestCalls)
        val removalReason = try {
            StockRemovalReason.valueOf(request.reason.lowercase())
        } catch (_: Exception) {
            StockRemovalReason.other
        }
        medicationsApi.createMedicationStockRemoval(
            householdId.toInt(),
            request.medicationId.toString(),
            StockRemovalRequest(
                StockRemovalRequestStockRemoval(
                    quantity = String.format(java.util.Locale.US, "%.2f", request.quantity),
                    reason = removalReason,
                    submissionId = UUID.randomUUID(),
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
        HealthEventsApi(apiBaseUrl(baseUrl), requestCalls).listHealthEvents(householdId.toInt()).data.map(HealthEvent::toDomain)
    }

    override suspend fun createHealthEvent(baseUrl: String, accessToken: String, householdId: Long, personId: Long, title: String, notes: String) = authenticated(accessToken) { requestCalls ->
        HealthEventsApi(apiBaseUrl(baseUrl), requestCalls).createHealthEvent(
            householdId.toInt(),
            HealthEventCreateRequest(
                HealthEventCreateRequestHealthEvent(
                    personId = personId.toString(),
                    eventKind = HealthEventCreateRequestHealthEvent.EventKind.suspected_side_effect,
                    title = title,
                    startedOn = LocalDate.now(),
                    notes = notes
                )
            )
        ).data.toDomain()
    }

    override suspend fun getAdminSettings(baseUrl: String, accessToken: String, householdId: Long) = authenticated(accessToken) { requestCalls ->
        HouseholdSettingsApi(apiBaseUrl(baseUrl), requestCalls).getHouseholdAdminSettings(householdId.toInt()).data.toDomain()
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
private fun HouseholdInvitation.toDomain() = HouseholdInvitationDto(id = id.toLong(), email = email, role = membershipRole.value, status = if (pending) "pending" else "accepted", expiresAt = expiresAt.toString())
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
private fun HouseholdAdminSettings.toDomain() = HouseholdAdminSettingsDto(subscriptionPlan = subscriptionPlan.value, operational = true)
