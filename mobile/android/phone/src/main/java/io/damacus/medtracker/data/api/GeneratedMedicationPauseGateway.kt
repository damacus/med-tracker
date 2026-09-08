package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.model.*
import io.medtracker.client.apis.*
import io.medtracker.client.models.MedicationPausePeriod
import io.medtracker.client.models.MedicationPausePeriodCreateRequest
import io.medtracker.client.models.MedicationPausePeriodCreateRequestMedicationPausePeriod
import io.medtracker.client.models.PaginationMeta
import java.util.UUID

class GeneratedMedicationPauseGateway(private val api: GeneratedMedTrackerApi = GeneratedMedTrackerApi()) : MedicationPauseGateway {
    override suspend fun supported(session: AppSession) = api.generated {
        val capability = CapabilitiesApi(api.apiBaseUrl(session.serverUrl), calls(session)).getCapabilities().data.medicationPausePeriods
        capability?.supported == true && capability.effectiveTime.value == "server_acceptance" &&
            capability.reasons.map { it.value }.containsAll(PauseReason.entries.map { it.value })
    }

    override suspend fun sources(session: AppSession) = api.generated {
        val base = api.apiBaseUrl(session.serverUrl)
        val calls = calls(session)
        val household = household(session)
        val people = pages { page -> PeopleApi(base, calls).listPeople(household, page, 100).let { it.data to it.meta } }.associateBy { it.id }
        val medicines = pages { page -> MedicationsApi(base, calls).listMedications(household, page, 100).let { it.data to it.meta } }.associateBy { it.id }
        val schedules = pages { page -> SchedulesApi(base, calls).listSchedules(household, page, 100).let { it.data to it.meta } }
        val assignments = pages { page -> PersonMedicationsApi(base, calls).listPersonMedications(household, page, 100).let { it.data to it.meta } }
        schedules.map {
            PauseSource("schedule", it.portableId.toString(), it.personId.toLong(), medicines[it.medicationId]?.name ?: "Medicine", it.paused,
                it.currentPausePeriod?.id?.toString(), people[it.personId]?.name.orEmpty(), it.active, it.currentPausePeriod?.reason?.value, it.currentPausePeriod?.note)
        } + assignments.map {
            PauseSource("person_medication", it.portableId.toString(), it.personId.toLong(), medicines[it.medicationId]?.name ?: "Medicine", it.paused,
                it.currentPausePeriod?.id?.toString(), people[it.personId]?.name.orEmpty(), it.active, it.currentPausePeriod?.reason?.value, it.currentPausePeriod?.note)
        }
    }

    override suspend fun history(session: AppSession, source: PauseSource) = api.generated {
        val client = SchedulesApi(api.apiBaseUrl(session.serverUrl), calls(session))
        pages { page -> client.listMedicationPausePeriods(household(session), page, 100,
            SchedulesApi.SourceTypeListMedicationPausePeriods.entries.first { it.value == source.type }, UUID.fromString(source.id)).let { it.data to it.meta }
        }.map { it.toPausePeriod() }
    }

    override suspend fun pause(session: AppSession, source: PauseSource, reason: PauseReason, note: String, requestId: String) = api.generated {
        SchedulesApi(api.apiBaseUrl(session.serverUrl), calls(session)).createMedicationPausePeriod(household(session),
            MedicationPausePeriodCreateRequest(MedicationPausePeriodCreateRequestMedicationPausePeriod(
                MedicationPausePeriodCreateRequestMedicationPausePeriod.SourceType.entries.first { it.value == source.type },
                UUID.fromString(source.id),
                MedicationPausePeriodCreateRequestMedicationPausePeriod.Reason.entries.first { it.value == reason.value }, note.takeIf { it.isNotBlank() }
            )), idempotencyKey = requestId
        ).data.toPausePeriod()
    }

    override suspend fun resume(session: AppSession, periodId: String) = api.generated {
        SchedulesApi(api.apiBaseUrl(session.serverUrl), calls(session)).resumeMedicationPausePeriod(household(session), UUID.fromString(periodId), idempotencyKey = UUID.nameUUIDFromBytes("resume:$periodId".toByteArray()).toString()).data.toPausePeriod()
    }

    private fun calls(session: AppSession) = RequestAuthCallFactory(api.callFactory, requireNotNull(session.accessToken))
    private fun household(session: AppSession) = requireNotNull(session.household?.id).toInt()

    private fun <T> pages(fetch: (Int) -> Pair<List<T>, PaginationMeta>): List<T> {
        val rows = mutableListOf<T>()
        var page = 1
        do {
            val (data, meta) = fetch(page)
            rows.addAll(data)
            if (data.isEmpty() || meta.page * meta.perPage >= meta.totalCount) break
            page++
        } while (true)
        return rows
    }
}

private fun MedicationPausePeriod.toPausePeriod() = PausePeriod(id.toString(), sourceType.value, sourceId.toString(), reason.value, note,
    startedAt?.toString(), endedAt?.toString(), legacyContext, recordedByName, resumedByName, createdAt.toString())
