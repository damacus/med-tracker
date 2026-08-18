# MedicationSuggestionsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**generateAiMedicationSuggestions**](MedicationSuggestionsAPI.md#generateaimedicationsuggestions) | **POST** /households/{household_id}/ai_medication_suggestions | Generate medication setup suggestions.


# **generateAiMedicationSuggestions**
```swift
    open class func generateAiMedicationSuggestions(householdId: Int, aiMedicationSuggestionRequest: AiMedicationSuggestionRequest? = nil, completion: @escaping (_ data: AiMedicationSuggestionResponse?, _ error: Error?) -> Void)
```

Generate medication setup suggestions.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let aiMedicationSuggestionRequest = AiMedicationSuggestionRequest(medication: AiMedicationIdentity(name: "name_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example")) // AiMedicationSuggestionRequest |  (optional)

// Generate medication setup suggestions.
MedicationSuggestionsAPI.generateAiMedicationSuggestions(householdId: householdId, aiMedicationSuggestionRequest: aiMedicationSuggestionRequest) { (response, error) in
    guard error == nil else {
        print(error)
        return
    }

    if (response) {
        dump(response)
    }
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **householdId** | **Int** |  | 
 **aiMedicationSuggestionRequest** | [**AiMedicationSuggestionRequest**](AiMedicationSuggestionRequest.md) |  | [optional] 

### Return type

[**AiMedicationSuggestionResponse**](AiMedicationSuggestionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

