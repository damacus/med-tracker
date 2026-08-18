# AppTokensApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createAppToken**](AppTokensApi.md#createAppToken) | **POST** /households/{household_id}/admin/app_tokens | Create an API app token. |
| [**deleteAppToken**](AppTokensApi.md#deleteAppToken) | **DELETE** /households/{household_id}/admin/app_tokens/{id} | Revoke an API app token. |
| [**listAppTokens**](AppTokensApi.md#listAppTokens) | **GET** /households/{household_id}/admin/app_tokens | List API app tokens. |


<a id="createAppToken"></a>
# **createAppToken**
> ApiAppTokenCreateResponse createAppToken(householdId, apiAppTokenCreateRequest)

Create an API app token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AppTokensApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val apiAppTokenCreateRequest : ApiAppTokenCreateRequest =  // ApiAppTokenCreateRequest | 
try {
    val result : ApiAppTokenCreateResponse = apiInstance.createAppToken(householdId, apiAppTokenCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AppTokensApi#createAppToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AppTokensApi#createAppToken")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **apiAppTokenCreateRequest** | [**ApiAppTokenCreateRequest**](ApiAppTokenCreateRequest.md)|  | |

### Return type

[**ApiAppTokenCreateResponse**](ApiAppTokenCreateResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="deleteAppToken"></a>
# **deleteAppToken**
> deleteAppToken(householdId, id)

Revoke an API app token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AppTokensApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteAppToken(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling AppTokensApi#deleteAppToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AppTokensApi#deleteAppToken")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
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

<a id="listAppTokens"></a>
# **listAppTokens**
> ApiAppTokenCollectionResponse listAppTokens(householdId)

List API app tokens.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AppTokensApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : ApiAppTokenCollectionResponse = apiInstance.listAppTokens(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AppTokensApi#listAppTokens")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AppTokensApi#listAppTokens")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**ApiAppTokenCollectionResponse**](ApiAppTokenCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

