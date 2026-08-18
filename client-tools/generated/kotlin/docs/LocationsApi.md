# LocationsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**getLocation**](LocationsApi.md#getLocation) | **GET** /households/{household_id}/locations/{id} | Read a location. |
| [**listLocations**](LocationsApi.md#listLocations) | **GET** /households/{household_id}/locations | List visible locations. |


<a id="getLocation"></a>
# **getLocation**
> LocationResponse getLocation(householdId, id)

Read a location.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = LocationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : LocationResponse = apiInstance.getLocation(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling LocationsApi#getLocation")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling LocationsApi#getLocation")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**LocationResponse**](LocationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listLocations"></a>
# **listLocations**
> LocationCollectionResponse listLocations(householdId, page, perPage, updatedSince)

List visible locations.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = LocationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : LocationCollectionResponse = apiInstance.listLocations(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling LocationsApi#listLocations")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling LocationsApi#listLocations")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **page** | **kotlin.Int**|  | [optional] [default to 1] |
| **perPage** | **kotlin.Int**|  | [optional] [default to 20] |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **updatedSince** | **java.time.OffsetDateTime**|  | [optional] |

### Return type

[**LocationCollectionResponse**](LocationCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

