package io.damacus.medtracker.data.model

import kotlinx.serialization.Serializable

data class OidcExchangeRequest(
    val idToken: String,
    val nonce: String,
    val codeVerifier: String,
    val deviceName: String? = null,
    val householdId: Long? = null
)

data class RefreshRequest(
    val refreshToken: String
)

@Serializable
data class SessionPayload(
    val accessToken: String,
    val accessTokenExpiresAt: String? = null,
    val refreshToken: String,
    val refreshTokenExpiresAt: String? = null,
    val me: UserDto? = null,
    val household: HouseholdDto? = null
)

@Serializable
data class UserDto(
    val id: Long? = null,
    val emailAddress: String? = null,
    val name: String? = null,
    val role: String? = null
)

@Serializable
data class HouseholdDto(
    val id: Long? = null,
    val name: String? = null
)
