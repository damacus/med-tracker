# DosageOptionsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createDosageOption**](DosageOptionsApi.md#createDosageOption) | **POST** /households/{household_id}/dosage_options | Create a medication dosage option. |
| [**getDosageOption**](DosageOptionsApi.md#getDosageOption) | **GET** /households/{household_id}/dosage_options/{id} | Read a medication dosage option. |
| [**listDosageOptions**](DosageOptionsApi.md#listDosageOptions) | **GET** /households/{household_id}/dosage_options | List medication dosage options. |
| [**replaceDosageOption**](DosageOptionsApi.md#replaceDosageOption) | **PUT** /households/{household_id}/dosage_options/{id} | Replace a medication dosage option. |
| [**updateDosageOption**](DosageOptionsApi.md#updateDosageOption) | **PATCH** /households/{household_id}/dosage_options/{id} | Update a medication dosage option. |


<a id="createDosageOption"></a>
# **createDosageOption**
> DosageOptionResponse createDosageOption(householdId, dosageOptionCreateRequest)

Create a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DosageOptionsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val dosageOptionCreateRequest : DosageOptionCreateRequest =  // DosageOptionCreateRequest | 
try {
    val result : DosageOptionResponse = apiInstance.createDosageOption(householdId, dosageOptionCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling DosageOptionsApi#createDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DosageOptionsApi#createDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **dosageOptionCreateRequest** | [**DosageOptionCreateRequest**](DosageOptionCreateRequest.md)|  | |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="getDosageOption"></a>
# **getDosageOption**
> DosageOptionResponse getDosageOption(householdId, id)

Read a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DosageOptionsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : DosageOptionResponse = apiInstance.getDosageOption(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling DosageOptionsApi#getDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DosageOptionsApi#getDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listDosageOptions"></a>
# **listDosageOptions**
> DosageOptionCollectionResponse listDosageOptions(householdId, page, perPage, updatedSince)

List medication dosage options.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DosageOptionsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : DosageOptionCollectionResponse = apiInstance.listDosageOptions(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling DosageOptionsApi#listDosageOptions")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DosageOptionsApi#listDosageOptions")
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

[**DosageOptionCollectionResponse**](DosageOptionCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="replaceDosageOption"></a>
# **replaceDosageOption**
> DosageOptionResponse replaceDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)

Replace a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DosageOptionsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val dosageOptionUpdateRequest : DosageOptionUpdateRequest =  // DosageOptionUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : DosageOptionResponse = apiInstance.replaceDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling DosageOptionsApi#replaceDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DosageOptionsApi#replaceDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updateDosageOption"></a>
# **updateDosageOption**
> DosageOptionResponse updateDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)

Update a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DosageOptionsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val dosageOptionUpdateRequest : DosageOptionUpdateRequest =  // DosageOptionUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : DosageOptionResponse = apiInstance.updateDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling DosageOptionsApi#updateDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DosageOptionsApi#updateDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

