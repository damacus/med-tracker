package io.damacus.medtracker.auth

import android.content.Context
import io.damacus.medtracker.data.SessionManager
import io.damacus.medtracker.data.api.MobileSessionTokenInterceptor
import io.damacus.medtracker.data.api.RefreshedMobileTokens
import net.openid.appauth.AuthState
import net.openid.appauth.AuthorizationService
import java.io.IOException
import java.time.Instant
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

object MobileSessionAuthentication {
    @Volatile var interceptor: MobileSessionTokenInterceptor? = null
        private set

    fun install(context: Context, sessions: SessionManager) {
        val service = AuthorizationService(context.applicationContext)
        interceptor = MobileSessionTokenInterceptor(sessions) { serialized ->
            val state = AuthState.jsonDeserialize(serialized)
            val completed = CountDownLatch(1)
            var tokens: RefreshedMobileTokens? = null
            state.performActionWithFreshTokens(service) { accessToken, _, error ->
                if (error == null && accessToken != null && state.refreshToken != null) {
                    tokens = RefreshedMobileTokens(accessToken, state.refreshToken!!, state.jsonSerializeString(),
                        state.accessTokenExpirationTime?.let { Instant.ofEpochMilli(it).toString() })
                }
                completed.countDown()
            }
            if (!completed.await(30, TimeUnit.SECONDS)) throw IOException("Session renewal timed out. Please retry.")
            tokens ?: throw IOException("Unable to renew this login. Retry or sign in again through MedTracker.")
        }
    }
}
