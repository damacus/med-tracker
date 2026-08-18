# HealthEventsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createHealthEvent**](HealthEventsAPI.md#createhealthevent) | **POST** /households/{household_id}/health_events | Create a health event.
[**getHealthEvent**](HealthEventsAPI.md#gethealthevent) | **GET** /households/{household_id}/health_events/{id} | Read a health event.
[**listHealthEvents**](HealthEventsAPI.md#listhealthevents) | **GET** /households/{household_id}/health_events | List visible health events.
[**replaceHealthEvent**](HealthEventsAPI.md#replacehealthevent) | **PUT** /households/{household_id}/health_events/{id} | Replace a health event.
[**updateHealthEvent**](HealthEventsAPI.md#updatehealthevent) | **PATCH** /households/{household_id}/health_events/{id} | Update a health event.


# **createHealthEvent**
```swift
    open class func createHealthEvent(householdId: Int, healthEventCreateRequest: HealthEventCreateRequest, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Create a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let healthEventCreateRequest = HealthEventCreateRequest(healthEvent: HealthEventCreateRequest_health_event(personId: "personId_example", eventKind: "eventKind_example", title: "title_example", startedOn: Date(), severity: "severity_example", notes: "notes_example", endedOn: Date(), medicationIds: ["medicationIds_example"])) // HealthEventCreateRequest | 

// Create a health event.
HealthEventsAPI.createHealthEvent(householdId: householdId, healthEventCreateRequest: healthEventCreateRequest) { (response, error) in
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
 **healthEventCreateRequest** | [**HealthEventCreateRequest**](HealthEventCreateRequest.md) |  | 

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getHealthEvent**
```swift
    open class func getHealthEvent(householdId: Int, id: String, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Read a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a health event.
HealthEventsAPI.getHealthEvent(householdId: householdId, id: id) { (response, error) in
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

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listHealthEvents**
```swift
    open class func listHealthEvents(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: HealthEventCollectionResponse?, _ error: Error?) -> Void)
```

List visible health events.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible health events.
HealthEventsAPI.listHealthEvents(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**HealthEventCollectionResponse**](HealthEventCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceHealthEvent**
```swift
    open class func replaceHealthEvent(householdId: Int, id: String, healthEventUpdateRequest: HealthEventUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Replace a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let healthEventUpdateRequest = HealthEventUpdateRequest(healthEvent: HealthEventAttributes(personId: "personId_example", eventKind: "eventKind_example", severity: "severity_example", title: "title_example", notes: "notes_example", startedOn: Date(), endedOn: Date(), medicationIds: ["medicationIds_example"])) // HealthEventUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a health event.
HealthEventsAPI.replaceHealthEvent(householdId: householdId, id: id, healthEventUpdateRequest: healthEventUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateHealthEvent**
```swift
    open class func updateHealthEvent(householdId: Int, id: String, healthEventUpdateRequest: HealthEventUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Update a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let healthEventUpdateRequest = HealthEventUpdateRequest(healthEvent: HealthEventAttributes(personId: "personId_example", eventKind: "eventKind_example", severity: "severity_example", title: "title_example", notes: "notes_example", startedOn: Date(), endedOn: Date(), medicationIds: ["medicationIds_example"])) // HealthEventUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a health event.
HealthEventsAPI.updateHealthEvent(householdId: householdId, id: id, healthEventUpdateRequest: healthEventUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

