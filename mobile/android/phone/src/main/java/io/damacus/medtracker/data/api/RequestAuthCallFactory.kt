package io.damacus.medtracker.data.api

import okhttp3.Call
import okhttp3.Request

internal class RequestAuthCallFactory(
    private val delegate: Call.Factory,
    private val accessToken: String? = null
) : Call.Factory {
    override fun newCall(request: Request): Call {
        val authenticatedRequest = request.newBuilder()
            .removeHeader("Authorization")
            .apply {
                if (!accessToken.isNullOrBlank()) header("Authorization", "Bearer $accessToken")
            }
            .build()
        return delegate.newCall(authenticatedRequest)
    }
}
