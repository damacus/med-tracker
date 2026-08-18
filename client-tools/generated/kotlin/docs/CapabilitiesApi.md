# CapabilitiesApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**getCapabilities**](CapabilitiesApi.md#getCapabilities) | **GET** /capabilities | Describe supported API features, diagnostics, and first-party client-tool contracts. |


<a id="getCapabilities"></a>
# **getCapabilities**
> CapabilitiesResponse getCapabilities()

Describe supported API features, diagnostics, and first-party client-tool contracts.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = CapabilitiesApi()
try {
    val result : CapabilitiesResponse = apiInstance.getCapabilities()
    println(result)
} catch (e: ClientException) {
    println("4xx response calling CapabilitiesApi#getCapabilities")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling CapabilitiesApi#getCapabilities")
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

