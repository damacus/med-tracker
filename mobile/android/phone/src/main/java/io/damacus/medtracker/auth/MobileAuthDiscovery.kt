package io.damacus.medtracker.auth

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.HttpUrl
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.OkHttpClient
import okhttp3.Request

data class MobileAuthConfiguration(
    val serverUrl: String,
    val clientId: String,
    val redirectUri: String,
    val authorizationEndpoint: String,
    val tokenEndpoint: String,
    val revocationEndpoint: String
)

class MobileAuthDiscovery(
    client: OkHttpClient = OkHttpClient(),
    private val allowLocalHttp: Boolean = false
) {
    private val client = client.newBuilder().followRedirects(false).followSslRedirects(false).build()

    suspend fun fetch(serverUrl: String, redirectUri: String): MobileAuthConfiguration = withContext(Dispatchers.IO) {
        val base = serverUrl.trim().toHttpUrl()
        require(base.username.isEmpty() && base.password.isEmpty() && base.query == null && base.fragment == null)
        require(base.encodedPath == "/") { "Enter the instance URL without a path." }
        require(base.isHttps || (allowLocalHttp && base.host in setOf("localhost", "127.0.0.1", "10.0.2.2", "::1"))) {
            "Use an HTTPS instance URL."
        }
        val capabilities = get(base.resolve("api/v1/capabilities")!!)
            .getValue("data").jsonObject.getValue("authentication").jsonObject
            .getValue("mobile_oauth").jsonObject
        require(capabilities.text("household_binding") == "account") { "This instance does not support account login." }
        val registered = capabilities.getValue("clients").jsonArray.map { it.jsonObject }
            .filter { redirectUri in it.strings("redirect_uris") }
            .singleOrNull() ?: error("This app is not registered on the selected instance.")
        require(registered.strings("scopes").containsAll(listOf("medtracker", "offline_access")))
        val discoveryUrl = ownedEndpoint(base, capabilities.text("discovery_url"))
        require(discoveryUrl.encodedPath == "/.well-known/oauth-authorization-server")
        val metadata = get(discoveryUrl)
        require(metadata.text("issuer").trimEnd('/') == base.toString().trimEnd('/')) { "The instance issuer does not match." }
        require("S256" in metadata.strings("code_challenge_methods_supported"))
        require(metadata.strings("grant_types_supported").containsAll(listOf("authorization_code", "refresh_token")))
        MobileAuthConfiguration(
            base.toString(), registered.text("client_id"), redirectUri,
            ownedEndpoint(base, metadata.text("authorization_endpoint")).toString(),
            ownedEndpoint(base, metadata.text("token_endpoint")).toString(),
            ownedEndpoint(base, metadata.text("revocation_endpoint")).toString()
        )
    }

    private fun ownedEndpoint(base: HttpUrl, value: String): HttpUrl {
        val endpoint = value.toHttpUrl()
        require(endpoint.scheme == base.scheme && endpoint.host == base.host && endpoint.port == base.port &&
            endpoint.username.isEmpty() && endpoint.password.isEmpty() && endpoint.query == null && endpoint.fragment == null) {
            "Authentication endpoints must belong to the selected instance."
        }
        return endpoint
    }

    private fun get(url: HttpUrl): JsonObject = client.newCall(Request.Builder().url(url).build()).execute().use { response ->
        require(response.isSuccessful) { "The instance does not provide supported authentication discovery." }
        val body = response.body ?: error("Empty discovery response.")
        require(body.contentLength() <= 65_536) { "Discovery response is too large." }
        val source = body.source()
        source.request(65_537)
        require(source.buffer.size <= 65_536) { "Discovery response is too large." }
        Json.parseToJsonElement(source.readUtf8()).jsonObject
    }

    private fun JsonObject.text(key: String) = getValue(key).jsonPrimitive.content
    private fun JsonObject.strings(key: String) = getValue(key).jsonArray.map { it.jsonPrimitive.content }
}
