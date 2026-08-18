# MedicationLookupApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**searchMedicationLookup**](MedicationLookupApi.md#searchMedicationLookup) | **GET** /households/{household_id}/medication_lookup | Search external medication lookup data. |


<a id="searchMedicationLookup"></a>
# **searchMedicationLookup**
> MedicationLookupResponse searchMedicationLookup(householdId, q, form, strength)

Search external medication lookup data.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationLookupApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val q : kotlin.String = q_example // kotlin.String | Medicine name, dm+d code, or barcode to search for.
val form : kotlin.String = form_example // kotlin.String | Dosage form used to filter search results.
val strength : kotlin.String = strength_example // kotlin.String | Medicine strength used to filter search results.
try {
    val result : MedicationLookupResponse = apiInstance.searchMedicationLookup(householdId, q, form, strength)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationLookupApi#searchMedicationLookup")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationLookupApi#searchMedicationLookup")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **q** | **kotlin.String**| Medicine name, dm+d code, or barcode to search for. | [optional] |
| **form** | **kotlin.String**| Dosage form used to filter search results. | [optional] |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **strength** | **kotlin.String**| Medicine strength used to filter search results. | [optional] |

### Return type

[**MedicationLookupResponse**](MedicationLookupResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

