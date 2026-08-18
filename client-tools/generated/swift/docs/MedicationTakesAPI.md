# MedicationTakesAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createMedicationTake**](MedicationTakesAPI.md#createmedicationtake) | **POST** /households/{household_id}/medication_takes | Record a medication take.
[**listMedicationTakes**](MedicationTakesAPI.md#listmedicationtakes) | **GET** /households/{household_id}/medication_takes | List visible medication takes.


# **createMedicationTake**
```swift
    open class func createMedicationTake(householdId: Int, medicationTakeCreateRequest: MedicationTakeCreateRequest, completion: @escaping (_ data: MedicationTakeResponse?, _ error: Error?) -> Void)
```

Record a medication take.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let medicationTakeCreateRequest = MedicationTakeCreateRequest(medicationTake: MedicationTakeCreateRequest_medication_take(sourceType: "sourceType_example", sourceId: "sourceId_example", takenAt: Date(), clientUuid: 123, doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", takenFromMedicationId: 123)) // MedicationTakeCreateRequest | 

// Record a medication take.
MedicationTakesAPI.createMedicationTake(householdId: householdId, medicationTakeCreateRequest: medicationTakeCreateRequest) { (response, error) in
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
 **medicationTakeCreateRequest** | [**MedicationTakeCreateRequest**](MedicationTakeCreateRequest.md) |  | 

### Return type

[**MedicationTakeResponse**](MedicationTakeResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listMedicationTakes**
```swift
    open class func listMedicationTakes(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: MedicationTakeCollectionResponse?, _ error: Error?) -> Void)
```

List visible medication takes.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible medication takes.
MedicationTakesAPI.listMedicationTakes(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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
 **page** | **Int** |  | [optional] [default to 1]
 **perPage** | **Int** |  | [optional] [default to 20]
 **updatedSince** | **Date** |  | [optional] 

### Return type

[**MedicationTakeCollectionResponse**](MedicationTakeCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

