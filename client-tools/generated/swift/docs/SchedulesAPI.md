# SchedulesAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createSchedule**](SchedulesAPI.md#createschedule) | **POST** /households/{household_id}/schedules | Create a schedule.
[**getSchedule**](SchedulesAPI.md#getschedule) | **GET** /households/{household_id}/schedules/{id} | Read a schedule.
[**listSchedules**](SchedulesAPI.md#listschedules) | **GET** /households/{household_id}/schedules | List visible schedules.
[**pauseSchedule**](SchedulesAPI.md#pauseschedule) | **PATCH** /households/{household_id}/schedules/{id}/pause | Pause a schedule.
[**replaceSchedule**](SchedulesAPI.md#replaceschedule) | **PUT** /households/{household_id}/schedules/{id} | Replace a schedule.
[**resumeSchedule**](SchedulesAPI.md#resumeschedule) | **PATCH** /households/{household_id}/schedules/{id}/resume | Resume a schedule.
[**updateSchedule**](SchedulesAPI.md#updateschedule) | **PATCH** /households/{household_id}/schedules/{id} | Update a schedule.


# **createSchedule**
```swift
    open class func createSchedule(householdId: Int, scheduleCreateRequest: ScheduleCreateRequest, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Create a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let scheduleCreateRequest = ScheduleCreateRequest(schedule: ScheduleCreateRequest_schedule(personId: "personId_example", medicationId: "medicationId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", startDate: Date(), endDate: Date(), sourceDosageOptionId: "sourceDosageOptionId_example", frequency: "frequency_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example", scheduleType: "scheduleType_example", scheduleConfig: ScheduleConfig(times: ["times_example"], weekdays: ["weekdays_example"], dates: [Date()], asNeeded: false, taperSteps: [ScheduleTaperStep(startDate: Date(), endDate: Date(), amount: "amount_example", doseAmount: "doseAmount_example", unit: "unit_example", doseUnit: "doseUnit_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", times: ["times_example"])]))) // ScheduleCreateRequest | 

// Create a schedule.
SchedulesAPI.createSchedule(householdId: householdId, scheduleCreateRequest: scheduleCreateRequest) { (response, error) in
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
 **scheduleCreateRequest** | [**ScheduleCreateRequest**](ScheduleCreateRequest.md) |  | 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSchedule**
```swift
    open class func getSchedule(householdId: Int, id: String, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Read a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a schedule.
SchedulesAPI.getSchedule(householdId: householdId, id: id) { (response, error) in
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

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listSchedules**
```swift
    open class func listSchedules(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: ScheduleCollectionResponse?, _ error: Error?) -> Void)
```

List visible schedules.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible schedules.
SchedulesAPI.listSchedules(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**ScheduleCollectionResponse**](ScheduleCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **pauseSchedule**
```swift
    open class func pauseSchedule(householdId: Int, id: String, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Pause a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Pause a schedule.
SchedulesAPI.pauseSchedule(householdId: householdId, id: id) { (response, error) in
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

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceSchedule**
```swift
    open class func replaceSchedule(householdId: Int, id: String, scheduleUpdateRequest: ScheduleUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Replace a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let scheduleUpdateRequest = ScheduleUpdateRequest(schedule: ScheduleAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", frequency: "frequency_example", startDate: Date(), endDate: Date(), notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example", scheduleType: "scheduleType_example", scheduleConfig: ScheduleConfig(times: ["times_example"], weekdays: ["weekdays_example"], dates: [Date()], asNeeded: false, taperSteps: [ScheduleTaperStep(startDate: Date(), endDate: Date(), amount: "amount_example", doseAmount: "doseAmount_example", unit: "unit_example", doseUnit: "doseUnit_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", times: ["times_example"])]))) // ScheduleUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a schedule.
SchedulesAPI.replaceSchedule(householdId: householdId, id: id, scheduleUpdateRequest: scheduleUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **resumeSchedule**
```swift
    open class func resumeSchedule(householdId: Int, id: String, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Resume a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Resume a schedule.
SchedulesAPI.resumeSchedule(householdId: householdId, id: id) { (response, error) in
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

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateSchedule**
```swift
    open class func updateSchedule(householdId: Int, id: String, scheduleUpdateRequest: ScheduleUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Update a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let scheduleUpdateRequest = ScheduleUpdateRequest(schedule: ScheduleAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", frequency: "frequency_example", startDate: Date(), endDate: Date(), notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example", scheduleType: "scheduleType_example", scheduleConfig: ScheduleConfig(times: ["times_example"], weekdays: ["weekdays_example"], dates: [Date()], asNeeded: false, taperSteps: [ScheduleTaperStep(startDate: Date(), endDate: Date(), amount: "amount_example", doseAmount: "doseAmount_example", unit: "unit_example", doseUnit: "doseUnit_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", times: ["times_example"])]))) // ScheduleUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a schedule.
SchedulesAPI.updateSchedule(householdId: householdId, id: id, scheduleUpdateRequest: scheduleUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

