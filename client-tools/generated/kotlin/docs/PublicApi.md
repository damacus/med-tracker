# PublicApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createLoginSession**](PublicApi.md#createLoginSession) | **POST** /auth/login | Exchange development or migration password credentials for API tokens. |
| [**exchangeOidcSession**](PublicApi.md#exchangeOidcSession) | **POST** /auth/oidc_exchange | Exchange a hosted OIDC ID token for an API session. |
| [**getCapabilities**](PublicApi.md#getCapabilities) | **GET** /capabilities | Describe supported API features, diagnostics, and first-party client-tool contracts. |


<a id="createLoginSession"></a>
# **createLoginSession**
> AuthLoginResponse createLoginSession(authLoginRequest)

Exchange development or migration password credentials for API tokens.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PublicApi()
val authLoginRequest : AuthLoginRequest =  // AuthLoginRequest | 
try {
    val result : AuthLoginResponse = apiInstance.createLoginSession(authLoginRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PublicApi#createLoginSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PublicApi#createLoginSession")
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

<a id="exchangeOidcSession"></a>
# **exchangeOidcSession**
> AuthLoginResponse exchangeOidcSession(authOidcExchangeRequest)

Exchange a hosted OIDC ID token for an API session.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PublicApi()
val authOidcExchangeRequest : AuthOidcExchangeRequest =  // AuthOidcExchangeRequest | 
try {
    val result : AuthLoginResponse = apiInstance.exchangeOidcSession(authOidcExchangeRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PublicApi#exchangeOidcSession")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PublicApi#exchangeOidcSession")
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

<a id="getCapabilities"></a>
# **getCapabilities**
> CapabilitiesResponse getCapabilities()

Describe supported API features, diagnostics, and first-party client-tool contracts.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PublicApi()
try {
    val result : CapabilitiesResponse = apiInstance.getCapabilities()
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PublicApi#getCapabilities")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PublicApi#getCapabilities")
    e.printStackTrace()
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**CapabilitiesResponse**](CapabilitiesResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

