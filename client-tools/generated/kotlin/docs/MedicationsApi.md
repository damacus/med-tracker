# MedicationsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**adjustMedicationInventory**](MedicationsApi.md#adjustMedicationInventory) | **PATCH** /households/{household_id}/medications/{id}/adjust_inventory | Adjust medication inventory. |
| [**createMedication**](MedicationsApi.md#createMedication) | **POST** /households/{household_id}/medications | Create a medication. |
| [**getMedication**](MedicationsApi.md#getMedication) | **GET** /households/{household_id}/medications/{id} | Read a medication. |
| [**listMedications**](MedicationsApi.md#listMedications) | **GET** /households/{household_id}/medications | List visible medications. |
| [**markMedicationAsOrdered**](MedicationsApi.md#markMedicationAsOrdered) | **PATCH** /households/{household_id}/medications/{id}/mark_as_ordered | Mark a medication reorder as ordered. |
| [**markMedicationAsReceived**](MedicationsApi.md#markMedicationAsReceived) | **PATCH** /households/{household_id}/medications/{id}/mark_as_received | Mark a medication reorder as received. |
| [**replaceMedication**](MedicationsApi.md#replaceMedication) | **PUT** /households/{household_id}/medications/{id} | Replace a medication. |
| [**updateMedication**](MedicationsApi.md#updateMedication) | **PATCH** /households/{household_id}/medications/{id} | Update a medication. |


<a id="adjustMedicationInventory"></a>
# **adjustMedicationInventory**
> MedicationResponse adjustMedicationInventory(householdId, id, medicationInventoryAdjustmentRequest)

Adjust medication inventory.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationInventoryAdjustmentRequest : MedicationInventoryAdjustmentRequest =  // MedicationInventoryAdjustmentRequest | 
try {
    val result : MedicationResponse = apiInstance.adjustMedicationInventory(householdId, id, medicationInventoryAdjustmentRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#adjustMedicationInventory")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#adjustMedicationInventory")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationInventoryAdjustmentRequest** | [**MedicationInventoryAdjustmentRequest**](MedicationInventoryAdjustmentRequest.md)|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createMedication"></a>
# **createMedication**
> MedicationResponse createMedication(householdId, medicationCreateRequest)

Create a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val medicationCreateRequest : MedicationCreateRequest =  // MedicationCreateRequest | 
try {
    val result : MedicationResponse = apiInstance.createMedication(householdId, medicationCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#createMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#createMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationCreateRequest** | [**MedicationCreateRequest**](MedicationCreateRequest.md)|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="getMedication"></a>
# **getMedication**
> MedicationResponse getMedication(householdId, id)

Read a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.getMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#getMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#getMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listMedications"></a>
# **listMedications**
> MedicationCollectionResponse listMedications(householdId, page, perPage, updatedSince)

List visible medications.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : MedicationCollectionResponse = apiInstance.listMedications(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#listMedications")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#listMedications")
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

[**MedicationCollectionResponse**](MedicationCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="markMedicationAsOrdered"></a>
# **markMedicationAsOrdered**
> MedicationResponse markMedicationAsOrdered(householdId, id, medicationOrderDetailsRequest)

Mark a medication reorder as ordered.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationOrderDetailsRequest : MedicationOrderDetailsRequest =  // MedicationOrderDetailsRequest | 
try {
    val result : MedicationResponse = apiInstance.markMedicationAsOrdered(householdId, id, medicationOrderDetailsRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#markMedicationAsOrdered")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#markMedicationAsOrdered")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationOrderDetailsRequest** | [**MedicationOrderDetailsRequest**](MedicationOrderDetailsRequest.md)|  | [optional] |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="markMedicationAsReceived"></a>
# **markMedicationAsReceived**
> MedicationResponse markMedicationAsReceived(householdId, id)

Mark a medication reorder as received.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.markMedicationAsReceived(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#markMedicationAsReceived")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#markMedicationAsReceived")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="replaceMedication"></a>
# **replaceMedication**
> MedicationResponse replaceMedication(householdId, id, medicationUpdateRequest, ifMatch)

Replace a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationUpdateRequest : MedicationUpdateRequest =  // MedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.replaceMedication(householdId, id, medicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#replaceMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#replaceMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updateMedication"></a>
# **updateMedication**
> MedicationResponse updateMedication(householdId, id, medicationUpdateRequest, ifMatch)

Update a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationUpdateRequest : MedicationUpdateRequest =  // MedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.updateMedication(householdId, id, medicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationsApi#updateMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationsApi#updateMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

