package io.damacus.medtracker.data.api

import io.damacus.medtracker.data.SessionManager
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.Interceptor
import okhttp3.Request
import okhttp3.Response
import java.io.IOException

data class RefreshedMobileTokens(val accessToken: String, val refreshToken: String, val state: String, val expiresAt: String?)

class MobileSessionTokenInterceptor(
    private val sessions: SessionManager,
    private val refresh: (String) -> RefreshedMobileTokens
) : Interceptor {
    private val previousTokens = mutableMapOf<String, String>()

    override fun intercept(chain: Interceptor.Chain): Response = chain.proceed(authorize(chain.request()))

    @Synchronized
    private fun authorize(request: Request): Request {
        val session = sessions.sessionState.value
        val payload = session.sessionPayload ?: return request
        val state = payload.oauthState ?: return request
        val offered = request.header("Authorization")?.removePrefix("Bearer ") ?: return request
        previousTokens.entries.removeAll { it.value != session.revision }
        if (offered != payload.accessToken && previousTokens[offered] != session.revision) return request
        val origin = session.serverUrl.toHttpUrl()
        if (request.url.scheme != origin.scheme || request.url.host != origin.host || request.url.port != origin.port) {
            throw IOException("The request does not belong to the signed-in instance.")
        }
        val tokens = refresh(state)
        if (sessions.sessionState.value.revision != session.revision) throw IOException("The login changed during refresh.")
        if (tokens.state != state || tokens.accessToken != payload.accessToken) {
            val updated = payload.copy(accessToken = tokens.accessToken, refreshToken = tokens.refreshToken,
                accessTokenExpiresAt = tokens.expiresAt, oauthState = tokens.state)
            if (!sessions.updatePayload(session.revision, updated)) throw IOException("The login changed during refresh.")
            previousTokens[payload.accessToken] = session.revision
        }
        return request.newBuilder().header("Authorization", "Bearer ${tokens.accessToken}").build()
    }
}
