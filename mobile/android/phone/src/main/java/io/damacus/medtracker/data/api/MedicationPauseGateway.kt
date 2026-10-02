package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.AppSession
import io.damacus.medtracker.data.model.*

interface MedicationPauseGateway {
    suspend fun supported(session: AppSession): ApiResult<Boolean>
    suspend fun sources(session: AppSession): ApiResult<List<PauseSource>>
    suspend fun history(session: AppSession, source: PauseSource): ApiResult<List<PausePeriod>>
    suspend fun pause(session: AppSession, source: PauseSource, reason: PauseReason, note: String, requestId: String): ApiResult<PausePeriod>
    suspend fun resume(session: AppSession, periodId: String): ApiResult<PausePeriod>
}
