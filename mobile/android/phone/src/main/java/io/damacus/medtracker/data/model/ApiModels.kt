package io.damacus.medtracker.data.model

import kotlinx.serialization.Serializable

data class HouseholdChoice(
    val id: Long,
    val name: String,
    val role: String
)

data class HouseholdSelection(val households: List<HouseholdChoice>)

@Serializable
data class SessionPayload(
    val accessToken: String,
    val accessTokenExpiresAt: String? = null,
    val refreshToken: String,
    val refreshTokenExpiresAt: String? = null,
    val me: UserDto? = null,
    val household: HouseholdDto? = null,
    val oauthState: String? = null
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
