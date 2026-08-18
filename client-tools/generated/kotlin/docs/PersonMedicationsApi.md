# PersonMedicationsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createPersonMedication**](PersonMedicationsApi.md#createPersonMedication) | **POST** /households/{household_id}/person_medications | Create a person medication assignment. |
| [**getPersonMedication**](PersonMedicationsApi.md#getPersonMedication) | **GET** /households/{household_id}/person_medications/{id} | Read a person medication assignment. |
| [**listPersonMedications**](PersonMedicationsApi.md#listPersonMedications) | **GET** /households/{household_id}/person_medications | List visible person medication assignments. |
| [**pausePersonMedication**](PersonMedicationsApi.md#pausePersonMedication) | **PATCH** /households/{household_id}/person_medications/{id}/pause | Pause a person medication assignment. |
| [**reorderPersonMedication**](PersonMedicationsApi.md#reorderPersonMedication) | **PATCH** /households/{household_id}/person_medications/{id}/reorder | Reorder a person medication assignment. |
| [**replacePersonMedication**](PersonMedicationsApi.md#replacePersonMedication) | **PUT** /households/{household_id}/person_medications/{id} | Replace a person medication assignment. |
| [**resumePersonMedication**](PersonMedicationsApi.md#resumePersonMedication) | **PATCH** /households/{household_id}/person_medications/{id}/resume | Resume a person medication assignment. |
| [**updatePersonMedication**](PersonMedicationsApi.md#updatePersonMedication) | **PATCH** /households/{household_id}/person_medications/{id} | Update a person medication assignment. |


<a id="createPersonMedication"></a>
# **createPersonMedication**
> PersonMedicationResponse createPersonMedication(householdId, personMedicationCreateRequest)

Create a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val personMedicationCreateRequest : PersonMedicationCreateRequest =  // PersonMedicationCreateRequest | 
try {
    val result : PersonMedicationResponse = apiInstance.createPersonMedication(householdId, personMedicationCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#createPersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#createPersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personMedicationCreateRequest** | [**PersonMedicationCreateRequest**](PersonMedicationCreateRequest.md)|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="getPersonMedication"></a>
# **getPersonMedication**
> PersonMedicationResponse getPersonMedication(householdId, id)

Read a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.getPersonMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#getPersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#getPersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listPersonMedications"></a>
# **listPersonMedications**
> PersonMedicationCollectionResponse listPersonMedications(householdId, page, perPage, updatedSince)

List visible person medication assignments.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : PersonMedicationCollectionResponse = apiInstance.listPersonMedications(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#listPersonMedications")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#listPersonMedications")
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

[**PersonMedicationCollectionResponse**](PersonMedicationCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="pausePersonMedication"></a>
# **pausePersonMedication**
> PersonMedicationResponse pausePersonMedication(householdId, id)

Pause a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.pausePersonMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#pausePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#pausePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="reorderPersonMedication"></a>
# **reorderPersonMedication**
> PersonMedicationResponse reorderPersonMedication(householdId, id, personMedicationReorderRequest)

Reorder a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val personMedicationReorderRequest : PersonMedicationReorderRequest =  // PersonMedicationReorderRequest | 
try {
    val result : PersonMedicationResponse = apiInstance.reorderPersonMedication(householdId, id, personMedicationReorderRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#reorderPersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#reorderPersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personMedicationReorderRequest** | [**PersonMedicationReorderRequest**](PersonMedicationReorderRequest.md)|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="replacePersonMedication"></a>
# **replacePersonMedication**
> PersonMedicationResponse replacePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)

Replace a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val personMedicationUpdateRequest : PersonMedicationUpdateRequest =  // PersonMedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.replacePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#replacePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#replacePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="resumePersonMedication"></a>
# **resumePersonMedication**
> PersonMedicationResponse resumePersonMedication(householdId, id)

Resume a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.resumePersonMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#resumePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#resumePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="updatePersonMedication"></a>
# **updatePersonMedication**
> PersonMedicationResponse updatePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)

Update a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PersonMedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val personMedicationUpdateRequest : PersonMedicationUpdateRequest =  // PersonMedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.updatePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PersonMedicationsApi#updatePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PersonMedicationsApi#updatePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

