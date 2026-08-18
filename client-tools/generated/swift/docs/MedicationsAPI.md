# MedicationsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**adjustMedicationInventory**](MedicationsAPI.md#adjustmedicationinventory) | **PATCH** /households/{household_id}/medications/{id}/adjust_inventory | Adjust medication inventory.
[**createMedication**](MedicationsAPI.md#createmedication) | **POST** /households/{household_id}/medications | Create a medication.
[**getMedication**](MedicationsAPI.md#getmedication) | **GET** /households/{household_id}/medications/{id} | Read a medication.
[**listMedications**](MedicationsAPI.md#listmedications) | **GET** /households/{household_id}/medications | List visible medications.
[**markMedicationAsOrdered**](MedicationsAPI.md#markmedicationasordered) | **PATCH** /households/{household_id}/medications/{id}/mark_as_ordered | Mark a medication reorder as ordered.
[**markMedicationAsReceived**](MedicationsAPI.md#markmedicationasreceived) | **PATCH** /households/{household_id}/medications/{id}/mark_as_received | Mark a medication reorder as received.
[**replaceMedication**](MedicationsAPI.md#replacemedication) | **PUT** /households/{household_id}/medications/{id} | Replace a medication.
[**updateMedication**](MedicationsAPI.md#updatemedication) | **PATCH** /households/{household_id}/medications/{id} | Update a medication.


# **adjustMedicationInventory**
```swift
    open class func adjustMedicationInventory(householdId: Int, id: String, medicationInventoryAdjustmentRequest: MedicationInventoryAdjustmentRequest, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Adjust medication inventory.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationInventoryAdjustmentRequest = MedicationInventoryAdjustmentRequest(adjustment: MedicationInventoryAdjustmentRequest_adjustment(newQuantity: "newQuantity_example", reason: "reason_example")) // MedicationInventoryAdjustmentRequest | 

// Adjust medication inventory.
MedicationsAPI.adjustMedicationInventory(householdId: householdId, id: id, medicationInventoryAdjustmentRequest: medicationInventoryAdjustmentRequest) { (response, error) in
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
 **medicationInventoryAdjustmentRequest** | [**MedicationInventoryAdjustmentRequest**](MedicationInventoryAdjustmentRequest.md) |  | 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createMedication**
```swift
    open class func createMedication(householdId: Int, medicationCreateRequest: MedicationCreateRequest, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Create a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let medicationCreateRequest = MedicationCreateRequest(medication: MedicationCreateAttributes(name: "name_example", reorderThreshold: "reorderThreshold_example", locationId: 123, friendlyName: "friendlyName_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", currentSupply: "currentSupply_example", warnings: "warnings_example", defaultScheduleType: "defaultScheduleType_example")) // MedicationCreateRequest | 

// Create a medication.
MedicationsAPI.createMedication(householdId: householdId, medicationCreateRequest: medicationCreateRequest) { (response, error) in
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
 **medicationCreateRequest** | [**MedicationCreateRequest**](MedicationCreateRequest.md) |  | 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getMedication**
```swift
    open class func getMedication(householdId: Int, id: String, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Read a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a medication.
MedicationsAPI.getMedication(householdId: householdId, id: id) { (response, error) in
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

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listMedications**
```swift
    open class func listMedications(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: MedicationCollectionResponse?, _ error: Error?) -> Void)
```

List visible medications.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible medications.
MedicationsAPI.listMedications(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**MedicationCollectionResponse**](MedicationCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **markMedicationAsOrdered**
```swift
    open class func markMedicationAsOrdered(householdId: Int, id: String, medicationOrderDetailsRequest: MedicationOrderDetailsRequest? = nil, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Mark a medication reorder as ordered.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationOrderDetailsRequest = MedicationOrderDetailsRequest(orderDetails: MedicationOrderDetailsRequest_order_details(supplier: "supplier_example", quantity: "quantity_example", expectedArrivalOn: Date())) // MedicationOrderDetailsRequest |  (optional)

// Mark a medication reorder as ordered.
MedicationsAPI.markMedicationAsOrdered(householdId: householdId, id: id, medicationOrderDetailsRequest: medicationOrderDetailsRequest) { (response, error) in
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
 **medicationOrderDetailsRequest** | [**MedicationOrderDetailsRequest**](MedicationOrderDetailsRequest.md) |  | [optional] 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **markMedicationAsReceived**
```swift
    open class func markMedicationAsReceived(householdId: Int, id: String, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Mark a medication reorder as received.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Mark a medication reorder as received.
MedicationsAPI.markMedicationAsReceived(householdId: householdId, id: id) { (response, error) in
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

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceMedication**
```swift
    open class func replaceMedication(householdId: Int, id: String, medicationUpdateRequest: MedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Replace a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationUpdateRequest = MedicationUpdateRequest(medication: MedicationUpdateAttributes(name: "name_example", friendlyName: "friendlyName_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example", warnings: "warnings_example", locationId: 123, defaultScheduleType: "defaultScheduleType_example")) // MedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a medication.
MedicationsAPI.replaceMedication(householdId: householdId, id: id, medicationUpdateRequest: medicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateMedication**
```swift
    open class func updateMedication(householdId: Int, id: String, medicationUpdateRequest: MedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Update a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationUpdateRequest = MedicationUpdateRequest(medication: MedicationUpdateAttributes(name: "name_example", friendlyName: "friendlyName_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example", warnings: "warnings_example", locationId: 123, defaultScheduleType: "defaultScheduleType_example")) // MedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a medication.
MedicationsAPI.updateMedication(householdId: householdId, id: id, medicationUpdateRequest: medicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

