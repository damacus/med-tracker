# MedicationTakesApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createMedicationTake**](MedicationTakesApi.md#createMedicationTake) | **POST** /households/{household_id}/medication_takes | Record a medication take. |
| [**listMedicationTakes**](MedicationTakesApi.md#listMedicationTakes) | **GET** /households/{household_id}/medication_takes | List visible medication takes. |


<a id="createMedicationTake"></a>
# **createMedicationTake**
> MedicationTakeResponse createMedicationTake(householdId, medicationTakeCreateRequest)

Record a medication take.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationTakesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val medicationTakeCreateRequest : MedicationTakeCreateRequest =  // MedicationTakeCreateRequest | 
try {
    val result : MedicationTakeResponse = apiInstance.createMedicationTake(householdId, medicationTakeCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationTakesApi#createMedicationTake")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationTakesApi#createMedicationTake")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationTakeCreateRequest** | [**MedicationTakeCreateRequest**](MedicationTakeCreateRequest.md)|  | |

### Return type

[**MedicationTakeResponse**](MedicationTakeResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="listMedicationTakes"></a>
# **listMedicationTakes**
> MedicationTakeCollectionResponse listMedicationTakes(householdId, page, perPage, updatedSince)

List visible medication takes.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationTakesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : MedicationTakeCollectionResponse = apiInstance.listMedicationTakes(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationTakesApi#listMedicationTakes")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationTakesApi#listMedicationTakes")
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

[**MedicationTakeCollectionResponse**](MedicationTakeCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

