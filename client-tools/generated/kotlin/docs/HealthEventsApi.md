# HealthEventsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createHealthEvent**](HealthEventsApi.md#createHealthEvent) | **POST** /households/{household_id}/health_events | Create a health event. |
| [**getHealthEvent**](HealthEventsApi.md#getHealthEvent) | **GET** /households/{household_id}/health_events/{id} | Read a health event. |
| [**listHealthEvents**](HealthEventsApi.md#listHealthEvents) | **GET** /households/{household_id}/health_events | List visible health events. |
| [**replaceHealthEvent**](HealthEventsApi.md#replaceHealthEvent) | **PUT** /households/{household_id}/health_events/{id} | Replace a health event. |
| [**updateHealthEvent**](HealthEventsApi.md#updateHealthEvent) | **PATCH** /households/{household_id}/health_events/{id} | Update a health event. |


<a id="createHealthEvent"></a>
# **createHealthEvent**
> HealthEventResponse createHealthEvent(householdId, healthEventCreateRequest)

Create a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HealthEventsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val healthEventCreateRequest : HealthEventCreateRequest =  // HealthEventCreateRequest | 
try {
    val result : HealthEventResponse = apiInstance.createHealthEvent(householdId, healthEventCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HealthEventsApi#createHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HealthEventsApi#createHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **healthEventCreateRequest** | [**HealthEventCreateRequest**](HealthEventCreateRequest.md)|  | |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="getHealthEvent"></a>
# **getHealthEvent**
> HealthEventResponse getHealthEvent(householdId, id)

Read a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HealthEventsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : HealthEventResponse = apiInstance.getHealthEvent(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HealthEventsApi#getHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HealthEventsApi#getHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listHealthEvents"></a>
# **listHealthEvents**
> HealthEventCollectionResponse listHealthEvents(householdId, page, perPage, updatedSince)

List visible health events.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HealthEventsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : HealthEventCollectionResponse = apiInstance.listHealthEvents(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HealthEventsApi#listHealthEvents")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HealthEventsApi#listHealthEvents")
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

[**HealthEventCollectionResponse**](HealthEventCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="replaceHealthEvent"></a>
# **replaceHealthEvent**
> HealthEventResponse replaceHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)

Replace a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HealthEventsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val healthEventUpdateRequest : HealthEventUpdateRequest =  // HealthEventUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : HealthEventResponse = apiInstance.replaceHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HealthEventsApi#replaceHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HealthEventsApi#replaceHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updateHealthEvent"></a>
# **updateHealthEvent**
> HealthEventResponse updateHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)

Update a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HealthEventsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val healthEventUpdateRequest : HealthEventUpdateRequest =  // HealthEventUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : HealthEventResponse = apiInstance.updateHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HealthEventsApi#updateHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HealthEventsApi#updateHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

