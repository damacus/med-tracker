# PersonMedicationsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createPersonMedication**](PersonMedicationsAPI.md#createpersonmedication) | **POST** /households/{household_id}/person_medications | Create a person medication assignment.
[**getPersonMedication**](PersonMedicationsAPI.md#getpersonmedication) | **GET** /households/{household_id}/person_medications/{id} | Read a person medication assignment.
[**listPersonMedications**](PersonMedicationsAPI.md#listpersonmedications) | **GET** /households/{household_id}/person_medications | List visible person medication assignments.
[**pausePersonMedication**](PersonMedicationsAPI.md#pausepersonmedication) | **PATCH** /households/{household_id}/person_medications/{id}/pause | Pause a person medication assignment.
[**reorderPersonMedication**](PersonMedicationsAPI.md#reorderpersonmedication) | **PATCH** /households/{household_id}/person_medications/{id}/reorder | Reorder a person medication assignment.
[**replacePersonMedication**](PersonMedicationsAPI.md#replacepersonmedication) | **PUT** /households/{household_id}/person_medications/{id} | Replace a person medication assignment.
[**resumePersonMedication**](PersonMedicationsAPI.md#resumepersonmedication) | **PATCH** /households/{household_id}/person_medications/{id}/resume | Resume a person medication assignment.
[**updatePersonMedication**](PersonMedicationsAPI.md#updatepersonmedication) | **PATCH** /households/{household_id}/person_medications/{id} | Update a person medication assignment.


# **createPersonMedication**
```swift
    open class func createPersonMedication(householdId: Int, personMedicationCreateRequest: PersonMedicationCreateRequest, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Create a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let personMedicationCreateRequest = PersonMedicationCreateRequest(personMedication: PersonMedicationCreateRequest_person_medication(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", administrationKind: "administrationKind_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example")) // PersonMedicationCreateRequest | 

// Create a person medication assignment.
PersonMedicationsAPI.createPersonMedication(householdId: householdId, personMedicationCreateRequest: personMedicationCreateRequest) { (response, error) in
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
 **personMedicationCreateRequest** | [**PersonMedicationCreateRequest**](PersonMedicationCreateRequest.md) |  | 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getPersonMedication**
```swift
    open class func getPersonMedication(householdId: Int, id: String, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Read a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a person medication assignment.
PersonMedicationsAPI.getPersonMedication(householdId: householdId, id: id) { (response, error) in
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

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPersonMedications**
```swift
    open class func listPersonMedications(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: PersonMedicationCollectionResponse?, _ error: Error?) -> Void)
```

List visible person medication assignments.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible person medication assignments.
PersonMedicationsAPI.listPersonMedications(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**PersonMedicationCollectionResponse**](PersonMedicationCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **pausePersonMedication**
```swift
    open class func pausePersonMedication(householdId: Int, id: String, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Pause a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Pause a person medication assignment.
PersonMedicationsAPI.pausePersonMedication(householdId: householdId, id: id) { (response, error) in
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

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **reorderPersonMedication**
```swift
    open class func reorderPersonMedication(householdId: Int, id: String, personMedicationReorderRequest: PersonMedicationReorderRequest, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Reorder a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let personMedicationReorderRequest = PersonMedicationReorderRequest(direction: "direction_example") // PersonMedicationReorderRequest | 

// Reorder a person medication assignment.
PersonMedicationsAPI.reorderPersonMedication(householdId: householdId, id: id, personMedicationReorderRequest: personMedicationReorderRequest) { (response, error) in
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
 **personMedicationReorderRequest** | [**PersonMedicationReorderRequest**](PersonMedicationReorderRequest.md) |  | 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replacePersonMedication**
```swift
    open class func replacePersonMedication(householdId: Int, id: String, personMedicationUpdateRequest: PersonMedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Replace a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let personMedicationUpdateRequest = PersonMedicationUpdateRequest(personMedication: PersonMedicationAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", administrationKind: "administrationKind_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example")) // PersonMedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a person medication assignment.
PersonMedicationsAPI.replacePersonMedication(householdId: householdId, id: id, personMedicationUpdateRequest: personMedicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **resumePersonMedication**
```swift
    open class func resumePersonMedication(householdId: Int, id: String, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Resume a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Resume a person medication assignment.
PersonMedicationsAPI.resumePersonMedication(householdId: householdId, id: id) { (response, error) in
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

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updatePersonMedication**
```swift
    open class func updatePersonMedication(householdId: Int, id: String, personMedicationUpdateRequest: PersonMedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Update a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let personMedicationUpdateRequest = PersonMedicationUpdateRequest(personMedication: PersonMedicationAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", administrationKind: "administrationKind_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example")) // PersonMedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a person medication assignment.
PersonMedicationsAPI.updatePersonMedication(householdId: householdId, id: id, personMedicationUpdateRequest: personMedicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

