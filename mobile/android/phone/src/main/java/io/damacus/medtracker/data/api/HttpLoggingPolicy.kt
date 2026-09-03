package io.damacus.medtracker.data.api

import io.damacus.medtracker.BuildConfig
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor

object HttpLoggingPolicy {
    val sensitiveHeaders = setOf(
        "Authorization",
        "Cookie",
        "Set-Cookie",
        "X-MedTracker-Portable-Passphrase"
    )

    fun interceptor(
        isRelease: Boolean,
        explicitOptIn: Boolean,
        logger: HttpLoggingInterceptor.Logger = HttpLoggingInterceptor.Logger.DEFAULT
    ): HttpLoggingInterceptor = HttpLoggingInterceptor(logger).apply {
        sensitiveHeaders.forEach(::redactHeader)
        level = if (!isRelease && explicitOptIn) {
            HttpLoggingInterceptor.Level.BASIC
        } else {
            HttpLoggingInterceptor.Level.NONE
        }
    }

    fun client(): OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(
            interceptor(
                isRelease = BuildConfig.IS_RELEASE_BUILD,
                explicitOptIn = BuildConfig.HTTP_LOGGING_OPT_IN
            )
        )
        .build()
}
