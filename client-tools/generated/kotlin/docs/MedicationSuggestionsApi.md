# MedicationSuggestionsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**generateAiMedicationSuggestions**](MedicationSuggestionsApi.md#generateAiMedicationSuggestions) | **POST** /households/{household_id}/ai_medication_suggestions | Generate medication setup suggestions. |


<a id="generateAiMedicationSuggestions"></a>
# **generateAiMedicationSuggestions**
> AiMedicationSuggestionResponse generateAiMedicationSuggestions(householdId, aiMedicationSuggestionRequest)

Generate medication setup suggestions.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MedicationSuggestionsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val aiMedicationSuggestionRequest : AiMedicationSuggestionRequest =  // AiMedicationSuggestionRequest | 
try {
    val result : AiMedicationSuggestionResponse = apiInstance.generateAiMedicationSuggestions(householdId, aiMedicationSuggestionRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MedicationSuggestionsApi#generateAiMedicationSuggestions")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MedicationSuggestionsApi#generateAiMedicationSuggestions")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **aiMedicationSuggestionRequest** | [**AiMedicationSuggestionRequest**](AiMedicationSuggestionRequest.md)|  | [optional] |

### Return type

[**AiMedicationSuggestionResponse**](AiMedicationSuggestionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

