# AccountApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**deleteSession**](AccountApi.md#deleteSession) | **DELETE** /auth/sessions/{id} | Revoke an API session. |
| [**listHouseholds**](AccountApi.md#listHouseholds) | **GET** /auth/households | List households available to the current API credential. |
| [**listSessions**](AccountApi.md#listSessions) | **GET** /auth/sessions | List active API sessions for the current account. |
| [**logoutSession**](AccountApi.md#logoutSession) | **DELETE** /auth/logout | Revoke the current API session or app token. |
| [**refreshSession**](AccountApi.md#refreshSession) | **POST** /auth/refresh | Rotate an API session refresh token. |


<a id="deleteSession"></a>
# **deleteSession**
> deleteSession(id)

Revoke an API session.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AccountApi()
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteSession(id)
} catch (e: ClientException) {
    println("4xx response calling AccountApi#deleteSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccountApi#deleteSession")
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

<a id="listHouseholds"></a>
# **listHouseholds**
> AuthHouseholdCollectionResponse listHouseholds()

List households available to the current API credential.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AccountApi()
try {
    val result : AuthHouseholdCollectionResponse = apiInstance.listHouseholds()
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AccountApi#listHouseholds")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccountApi#listHouseholds")
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

val apiInstance = AccountApi()
try {
    val result : AuthSessionCollectionResponse = apiInstance.listSessions()
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AccountApi#listSessions")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccountApi#listSessions")
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

val apiInstance = AccountApi()
try {
    apiInstance.logoutSession()
} catch (e: ClientException) {
    println("4xx response calling AccountApi#logoutSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccountApi#logoutSession")
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

val apiInstance = AccountApi()
val authRefreshRequest : AuthRefreshRequest =  // AuthRefreshRequest | 
try {
    val result : AuthRefreshResponse = apiInstance.refreshSession(authRefreshRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AccountApi#refreshSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccountApi#refreshSession")
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

