# AuthenticationApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createLoginSession**](AuthenticationApi.md#createLoginSession) | **POST** /auth/login | Exchange development or migration password credentials for API tokens. |
| [**deleteSession**](AuthenticationApi.md#deleteSession) | **DELETE** /auth/sessions/{id} | Revoke an API session. |
| [**exchangeOidcSession**](AuthenticationApi.md#exchangeOidcSession) | **POST** /auth/oidc_exchange | Exchange a hosted OIDC ID token for an API session. |
| [**listHouseholds**](AuthenticationApi.md#listHouseholds) | **GET** /auth/households | List households available to the current API credential. |
| [**listSessions**](AuthenticationApi.md#listSessions) | **GET** /auth/sessions | List active API sessions for the current account. |
| [**logoutSession**](AuthenticationApi.md#logoutSession) | **DELETE** /auth/logout | Revoke the current API session or app token. |
| [**refreshSession**](AuthenticationApi.md#refreshSession) | **POST** /auth/refresh | Rotate an API session refresh token. |


<a id="createLoginSession"></a>
# **createLoginSession**
> AuthLoginResponse createLoginSession(authLoginRequest)

Exchange development or migration password credentials for API tokens.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuthenticationApi()
val authLoginRequest : AuthLoginRequest =  // AuthLoginRequest | 
try {
    val result : AuthLoginResponse = apiInstance.createLoginSession(authLoginRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AuthenticationApi#createLoginSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuthenticationApi#createLoginSession")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **authLoginRequest** | [**AuthLoginRequest**](AuthLoginRequest.md)|  | |

### Return type

[**AuthLoginResponse**](AuthLoginResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="deleteSession"></a>
# **deleteSession**
> deleteSession(id)

Revoke an API session.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuthenticationApi()
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteSession(id)
} catch (e: ClientException) {
    println("4xx response calling AuthenticationApi#deleteSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuthenticationApi#deleteSession")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int**|  | |

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="exchangeOidcSession"></a>
# **exchangeOidcSession**
> AuthLoginResponse exchangeOidcSession(authOidcExchangeRequest)

Exchange a hosted OIDC ID token for an API session.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuthenticationApi()
val authOidcExchangeRequest : AuthOidcExchangeRequest =  // AuthOidcExchangeRequest | 
try {
    val result : AuthLoginResponse = apiInstance.exchangeOidcSession(authOidcExchangeRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AuthenticationApi#exchangeOidcSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuthenticationApi#exchangeOidcSession")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **authOidcExchangeRequest** | [**AuthOidcExchangeRequest**](AuthOidcExchangeRequest.md)|  | |

### Return type

[**AuthLoginResponse**](AuthLoginResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="listHouseholds"></a>
# **listHouseholds**
> AuthHouseholdCollectionResponse listHouseholds()

List households available to the current API credential.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuthenticationApi()
try {
    val result : AuthHouseholdCollectionResponse = apiInstance.listHouseholds()
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AuthenticationApi#listHouseholds")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuthenticationApi#listHouseholds")
    e.printStackTrace()
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**AuthHouseholdCollectionResponse**](AuthHouseholdCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listSessions"></a>
# **listSessions**
> AuthSessionCollectionResponse listSessions()

List active API sessions for the current account.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuthenticationApi()
try {
    val result : AuthSessionCollectionResponse = apiInstance.listSessions()
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AuthenticationApi#listSessions")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuthenticationApi#listSessions")
    e.printStackTrace()
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**AuthSessionCollectionResponse**](AuthSessionCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="logoutSession"></a>
# **logoutSession**
> logoutSession()

Revoke the current API session or app token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuthenticationApi()
try {
    apiInstance.logoutSession()
} catch (e: ClientException) {
    println("4xx response calling AuthenticationApi#logoutSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuthenticationApi#logoutSession")
    e.printStackTrace()
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="refreshSession"></a>
# **refreshSession**
> AuthRefreshResponse refreshSession(authRefreshRequest)

Rotate an API session refresh token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuthenticationApi()
val authRefreshRequest : AuthRefreshRequest =  // AuthRefreshRequest | 
try {
    val result : AuthRefreshResponse = apiInstance.refreshSession(authRefreshRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AuthenticationApi#refreshSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuthenticationApi#refreshSession")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **authRefreshRequest** | [**AuthRefreshRequest**](AuthRefreshRequest.md)|  | |

### Return type

[**AuthRefreshResponse**](AuthRefreshResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

