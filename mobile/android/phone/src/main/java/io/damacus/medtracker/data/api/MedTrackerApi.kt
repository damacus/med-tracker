package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.model.HouseholdDto
import io.damacus.medtracker.data.model.MedicationDto
import io.damacus.medtracker.data.model.MedicationTakeDto
import io.damacus.medtracker.data.model.OidcExchangeRequest
import io.damacus.medtracker.data.model.PersonDto
import io.damacus.medtracker.data.model.RecordDosePayload
import io.damacus.medtracker.data.model.RefreshRequest
import io.damacus.medtracker.data.model.ScheduleDto
import io.damacus.medtracker.data.model.SessionPayload
import io.damacus.medtracker.data.model.UserDto
import io.medtracker.client.apis.AuthenticationApi
import io.medtracker.client.apis.MedicationTakesApi
import io.medtracker.client.apis.MedicationsApi
import io.medtracker.client.apis.PeopleApi
import io.medtracker.client.apis.SchedulesApi
import io.medtracker.client.infrastructure.ClientException
import io.medtracker.client.infrastructure.ServerException
import io.medtracker.client.models.AuthLoginData
import io.medtracker.client.models.AuthOidcExchangeRequest
import io.medtracker.client.models.AuthRefreshRequest
import io.medtracker.client.models.Medication
import io.medtracker.client.models.MedicationTake
import io.medtracker.client.models.MedicationTakeCreateRequest
import io.medtracker.client.models.MedicationTakeCreateRequestMedicationTake
import io.medtracker.client.models.Person
import io.medtracker.client.models.Schedule
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Call
import java.io.IOException
import java.time.OffsetDateTime
import java.util.UUID

sealed class ApiResult<out T> {
    data class Success<T>(val data: T) : ApiResult<T>()
    data class Error(val code: String?, val message: String, val statusCode: Int? = null) : ApiResult<Nothing>()
    data class NetworkError(val cause: Throwable) : ApiResult<Nothing>()
}

interface MedTrackerApi {
    suspend fun exchangeOidc(baseUrl: String, request: OidcExchangeRequest): ApiResult<SessionPayload>
    suspend fun refresh(baseUrl: String, request: RefreshRequest): ApiResult<SessionPayload>
    suspend fun logout(baseUrl: String, accessToken: String): ApiResult<Unit>
    suspend fun getPeople(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<PersonDto>>
    suspend fun getMedications(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationDto>>
    suspend fun getSchedules(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<ScheduleDto>>
    suspend fun getMedicationTakes(baseUrl: String, accessToken: String, householdId: Long): ApiResult<List<MedicationTakeDto>>
    suspend fun recordDose(baseUrl: String, accessToken: String, householdId: Long, request: RecordDosePayload): ApiResult<MedicationTakeDto>
}

class GeneratedMedTrackerApi(
    internal val callFactory: Call.Factory = HttpLoggingPolicy.client()
) : MedTrackerApi {
    private val unauthenticatedCalls = RequestAuthCallFactory(callFactory)

    override suspend fun exchangeOidc(baseUrl: String, request: OidcExchangeRequest) = generated {
        AuthenticationApi(apiBaseUrl(baseUrl), unauthenticatedCalls).exchangeOidcSession(
            AuthOidcExchangeRequest(request.idToken, request.nonce, request.codeVerifier, request.deviceName, request.householdId?.toInt())
        ).data.toSessionPayload()
    }

    override suspend fun refresh(baseUrl: String, request: RefreshRequest) = generated {
        AuthenticationApi(apiBaseUrl(baseUrl), unauthenticatedCalls).refreshSession(AuthRefreshRequest(request.refreshToken)).data.toSessionPayload()
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

    internal suspend fun <T> generated(block: () -> T): ApiResult<T> = withContext(Dispatchers.IO) {
        try { ApiResult.Success(block()) } catch (error: ClientException) { error.toResult() }
        catch (error: ServerException) { error.toResult() } catch (error: IOException) { ApiResult.NetworkError(error) }
    }

    private suspend fun <T> authenticated(accessToken: String, block: (Call.Factory) -> T): ApiResult<T> = generated {
        block(RequestAuthCallFactory(callFactory, accessToken))
    }

    private fun ClientException.toResult() = ApiResult.Error("http_$statusCode", message.orEmpty(), statusCode)
    private fun ServerException.toResult() = ApiResult.Error("http_$statusCode", message.orEmpty(), statusCode)
    internal fun apiBaseUrl(baseUrl: String): String = "${baseUrl.trimEnd('/')}/api/v1"

}

internal fun AuthLoginData.toSessionPayload() = SessionPayload(accessToken, accessTokenExpiresAt.toString(), refreshToken, refreshTokenExpiresAt.toString(), UserDto(me.id.toLong(), me.emailAddress, me.person.name, me.membershipRole?.value), household?.let { HouseholdDto(it.id.toLong(), it.name) })
private fun io.medtracker.client.models.AuthRefreshData.toSessionPayload() = SessionPayload(accessToken, accessTokenExpiresAt.toString(), refreshToken, refreshTokenExpiresAt.toString(), household = household?.let { HouseholdDto(it.id.toLong(), it.name) })
private fun Person.toDomain() = PersonDto(id.toLong(), portableId.toString(), name, email, dateOfBirth?.toString(), personType.value, age, hasCapacity)
private fun Medication.toDomain() = MedicationDto(id.toLong(), portableId.toString(), name, displayName, category, description, doseAmount?.toDoubleOrNull(), doseUnit, currentSupply?.toDoubleOrNull(), reorderThreshold.toDoubleOrNull(), reorderStatus?.value, lowStock, outOfStock)
private fun Schedule.toDomain() = ScheduleDto(id.toLong(), portableId.toString(), personId.toLong(), personPortableId.toString(), medicationId.toLong(), medicationPortableId.toString(), doseAmount.toDoubleOrNull(), doseUnit, frequency, doseCycle?.value, startDate.toString(), endDate.toString(), active, paused, notes, maxDailyDoses, minHoursBetweenDoses?.toDoubleOrNull())
private fun MedicationTake.toDomain() = MedicationTakeDto(id.toLong(), portableId.toString(), clientUuid?.toString(), scheduleId?.toLong(), personMedicationId?.toLong(), personId?.toLong(), medicationId?.toLong(), doseAmount?.toDoubleOrNull(), doseUnit, takenAt?.toString())
