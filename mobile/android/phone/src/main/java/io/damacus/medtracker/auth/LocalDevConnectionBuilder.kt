package io.damacus.medtracker.auth

import android.net.Uri
import net.openid.appauth.AppAuthConfiguration
import net.openid.appauth.connectivity.ConnectionBuilder
import net.openid.appauth.connectivity.DefaultConnectionBuilder
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import java.util.concurrent.TimeUnit

class LocalDevConnectionBuilder(
    private val allowLocalHttp: Boolean,
    private val delegate: ConnectionBuilder = DefaultConnectionBuilder.INSTANCE
) : ConnectionBuilder {

    @Throws(IOException::class)
    override fun openConnection(uri: Uri): HttpURLConnection {
        val raw = uri.toString()
        return if (isAllowedLocalHttpUrl(raw)) {
            openLocalHttpConnection(raw)
        } else {
            delegate.openConnection(uri)
        }
    }

    internal fun isAllowedLocalHttpUrl(rawUrl: String): Boolean {
        if (!allowLocalHttp) return false
        val parsed = runCatching { URI(rawUrl) }.getOrNull() ?: return false
        if (!parsed.scheme.equals("http", ignoreCase = true)) return false
        val host = parsed.host?.lowercase()?.trim('[', ']') ?: return false
        return host in LOCAL_HTTP_HOSTS
    }

    @Throws(IOException::class)
    internal fun openLocalHttpConnection(rawUrl: String): HttpURLConnection {
        require(isAllowedLocalHttpUrl(rawUrl)) { "Only HTTPS connections are permitted." }
        val connection = URL(rawUrl).openConnection() as HttpURLConnection
        connection.connectTimeout = CONNECTION_TIMEOUT_MS
        connection.readTimeout = READ_TIMEOUT_MS
        connection.instanceFollowRedirects = false
        return connection
    }

    companion object {
        private val LOCAL_HTTP_HOSTS = setOf("localhost", "127.0.0.1", "10.0.2.2", "::1")
        private val CONNECTION_TIMEOUT_MS = TimeUnit.SECONDS.toMillis(15).toInt()
        private val READ_TIMEOUT_MS = TimeUnit.SECONDS.toMillis(10).toInt()

        fun appAuthConfiguration(allowLocalHttp: Boolean): AppAuthConfiguration =
            AppAuthConfiguration.Builder()
                .setConnectionBuilder(LocalDevConnectionBuilder(allowLocalHttp))
                .build()
    }
}
