# DosageOptionsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createDosageOption**](DosageOptionsAPI.md#createdosageoption) | **POST** /households/{household_id}/dosage_options | Create a medication dosage option.
[**getDosageOption**](DosageOptionsAPI.md#getdosageoption) | **GET** /households/{household_id}/dosage_options/{id} | Read a medication dosage option.
[**listDosageOptions**](DosageOptionsAPI.md#listdosageoptions) | **GET** /households/{household_id}/dosage_options | List medication dosage options.
[**replaceDosageOption**](DosageOptionsAPI.md#replacedosageoption) | **PUT** /households/{household_id}/dosage_options/{id} | Replace a medication dosage option.
[**updateDosageOption**](DosageOptionsAPI.md#updatedosageoption) | **PATCH** /households/{household_id}/dosage_options/{id} | Update a medication dosage option.


# **createDosageOption**
```swift
    open class func createDosageOption(householdId: Int, dosageOptionCreateRequest: DosageOptionCreateRequest, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Create a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let dosageOptionCreateRequest = DosageOptionCreateRequest(dosageOption: DosageOptionCreateAttributes(medicationId: "medicationId_example", amount: "amount_example", unit: "unit_example", frequency: "frequency_example", defaultMaxDailyDoses: 123, defaultMinHoursBetweenDoses: "defaultMinHoursBetweenDoses_example", defaultDoseCycle: "defaultDoseCycle_example", description: "description_example", defaultForAdults: false, defaultForChildren: false, currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example")) // DosageOptionCreateRequest | 

// Create a medication dosage option.
DosageOptionsAPI.createDosageOption(householdId: householdId, dosageOptionCreateRequest: dosageOptionCreateRequest) { (response, error) in
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
 **dosageOptionCreateRequest** | [**DosageOptionCreateRequest**](DosageOptionCreateRequest.md) |  | 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getDosageOption**
```swift
    open class func getDosageOption(householdId: Int, id: String, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Read a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a medication dosage option.
DosageOptionsAPI.getDosageOption(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listDosageOptions**
```swift
    open class func listDosageOptions(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: DosageOptionCollectionResponse?, _ error: Error?) -> Void)
```

List medication dosage options.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List medication dosage options.
DosageOptionsAPI.listDosageOptions(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**DosageOptionCollectionResponse**](DosageOptionCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceDosageOption**
```swift
    open class func replaceDosageOption(householdId: Int, id: String, dosageOptionUpdateRequest: DosageOptionUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Replace a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let dosageOptionUpdateRequest = DosageOptionUpdateRequest(dosageOption: DosageOptionUpdateAttributes(amount: "amount_example", unit: "unit_example", frequency: "frequency_example", description: "description_example", defaultForAdults: false, defaultForChildren: false, defaultMaxDailyDoses: 123, defaultMinHoursBetweenDoses: "defaultMinHoursBetweenDoses_example", defaultDoseCycle: "defaultDoseCycle_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example")) // DosageOptionUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a medication dosage option.
DosageOptionsAPI.replaceDosageOption(householdId: householdId, id: id, dosageOptionUpdateRequest: dosageOptionUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateDosageOption**
```swift
    open class func updateDosageOption(householdId: Int, id: String, dosageOptionUpdateRequest: DosageOptionUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Update a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let dosageOptionUpdateRequest = DosageOptionUpdateRequest(dosageOption: DosageOptionUpdateAttributes(amount: "amount_example", unit: "unit_example", frequency: "frequency_example", description: "description_example", defaultForAdults: false, defaultForChildren: false, defaultMaxDailyDoses: 123, defaultMinHoursBetweenDoses: "defaultMinHoursBetweenDoses_example", defaultDoseCycle: "defaultDoseCycle_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example")) // DosageOptionUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a medication dosage option.
DosageOptionsAPI.updateDosageOption(householdId: householdId, id: id, dosageOptionUpdateRequest: dosageOptionUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

