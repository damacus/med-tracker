# MedicationLookupAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**searchMedicationLookup**](MedicationLookupAPI.md#searchmedicationlookup) | **GET** /households/{household_id}/medication_lookup | Search external medication lookup data.


# **searchMedicationLookup**
```swift
    open class func searchMedicationLookup(householdId: Int, q: String? = nil, form: String? = nil, strength: String? = nil, completion: @escaping (_ data: MedicationLookupResponse?, _ error: Error?) -> Void)
```

Search external medication lookup data.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let q = "q_example" // String | Medicine name, dm+d code, or barcode to search for. (optional)
let form = "form_example" // String | Dosage form used to filter search results. (optional)
let strength = "strength_example" // String | Medicine strength used to filter search results. (optional)

// Search external medication lookup data.
MedicationLookupAPI.searchMedicationLookup(householdId: householdId, q: q, form: form, strength: strength) { (response, error) in
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
 **q** | **String** | Medicine name, dm+d code, or barcode to search for. | [optional] 
 **form** | **String** | Dosage form used to filter search results. | [optional] 
 **strength** | **String** | Medicine strength used to filter search results. | [optional] 

### Return type

[**MedicationLookupResponse**](MedicationLookupResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

