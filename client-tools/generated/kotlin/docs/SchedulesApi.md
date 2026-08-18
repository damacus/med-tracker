# SchedulesApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createSchedule**](SchedulesApi.md#createSchedule) | **POST** /households/{household_id}/schedules | Create a schedule. |
| [**getSchedule**](SchedulesApi.md#getSchedule) | **GET** /households/{household_id}/schedules/{id} | Read a schedule. |
| [**listSchedules**](SchedulesApi.md#listSchedules) | **GET** /households/{household_id}/schedules | List visible schedules. |
| [**pauseSchedule**](SchedulesApi.md#pauseSchedule) | **PATCH** /households/{household_id}/schedules/{id}/pause | Pause a schedule. |
| [**replaceSchedule**](SchedulesApi.md#replaceSchedule) | **PUT** /households/{household_id}/schedules/{id} | Replace a schedule. |
| [**resumeSchedule**](SchedulesApi.md#resumeSchedule) | **PATCH** /households/{household_id}/schedules/{id}/resume | Resume a schedule. |
| [**updateSchedule**](SchedulesApi.md#updateSchedule) | **PATCH** /households/{household_id}/schedules/{id} | Update a schedule. |


<a id="createSchedule"></a>
# **createSchedule**
> ScheduleResponse createSchedule(householdId, scheduleCreateRequest)

Create a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SchedulesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val scheduleCreateRequest : ScheduleCreateRequest =  // ScheduleCreateRequest | 
try {
    val result : ScheduleResponse = apiInstance.createSchedule(householdId, scheduleCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SchedulesApi#createSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SchedulesApi#createSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **scheduleCreateRequest** | [**ScheduleCreateRequest**](ScheduleCreateRequest.md)|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="getSchedule"></a>
# **getSchedule**
> ScheduleResponse getSchedule(householdId, id)

Read a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SchedulesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.getSchedule(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SchedulesApi#getSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SchedulesApi#getSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listSchedules"></a>
# **listSchedules**
> ScheduleCollectionResponse listSchedules(householdId, page, perPage, updatedSince)

List visible schedules.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SchedulesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : ScheduleCollectionResponse = apiInstance.listSchedules(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SchedulesApi#listSchedules")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SchedulesApi#listSchedules")
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

[**ScheduleCollectionResponse**](ScheduleCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="pauseSchedule"></a>
# **pauseSchedule**
> ScheduleResponse pauseSchedule(householdId, id)

Pause a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SchedulesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.pauseSchedule(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SchedulesApi#pauseSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SchedulesApi#pauseSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="replaceSchedule"></a>
# **replaceSchedule**
> ScheduleResponse replaceSchedule(householdId, id, scheduleUpdateRequest, ifMatch)

Replace a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SchedulesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val scheduleUpdateRequest : ScheduleUpdateRequest =  // ScheduleUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.replaceSchedule(householdId, id, scheduleUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SchedulesApi#replaceSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SchedulesApi#replaceSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="resumeSchedule"></a>
# **resumeSchedule**
> ScheduleResponse resumeSchedule(householdId, id)

Resume a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SchedulesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.resumeSchedule(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SchedulesApi#resumeSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SchedulesApi#resumeSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="updateSchedule"></a>
# **updateSchedule**
> ScheduleResponse updateSchedule(householdId, id, scheduleUpdateRequest, ifMatch)

Update a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SchedulesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val scheduleUpdateRequest : ScheduleUpdateRequest =  // ScheduleUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.updateSchedule(householdId, id, scheduleUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SchedulesApi#updateSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SchedulesApi#updateSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

