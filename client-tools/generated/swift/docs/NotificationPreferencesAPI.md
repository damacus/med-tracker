# NotificationPreferencesAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getNotificationPreference**](NotificationPreferencesAPI.md#getnotificationpreference) | **GET** /households/{household_id}/notification_preference | Read the signed-in person&#39;s notification preference.
[**replaceNotificationPreference**](NotificationPreferencesAPI.md#replacenotificationpreference) | **PUT** /households/{household_id}/notification_preference | Replace the signed-in person&#39;s notification preference.
[**updateNotificationPreference**](NotificationPreferencesAPI.md#updatenotificationpreference) | **PATCH** /households/{household_id}/notification_preference | Update the signed-in person&#39;s notification preference.


# **getNotificationPreference**
```swift
    open class func getNotificationPreference(householdId: Int, completion: @escaping (_ data: NotificationPreferenceResponse?, _ error: Error?) -> Void)
```

Read the signed-in person's notification preference.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read the signed-in person's notification preference.
NotificationPreferencesAPI.getNotificationPreference(householdId: householdId) { (response, error) in
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

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceNotificationPreference**
```swift
    open class func replaceNotificationPreference(householdId: Int, notificationPreferenceUpdateRequest: NotificationPreferenceUpdateRequest, completion: @escaping (_ data: NotificationPreferenceResponse?, _ error: Error?) -> Void)
```

Replace the signed-in person's notification preference.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let notificationPreferenceUpdateRequest = NotificationPreferenceUpdateRequest(notificationPreference: NotificationPreferenceAttributes(enabled: false, doseDueEnabled: false, missedDoseEnabled: false, lowStockEnabled: false, privateTextEnabled: false, morningTime: "morningTime_example", afternoonTime: "afternoonTime_example", eveningTime: "eveningTime_example", nightTime: "nightTime_example")) // NotificationPreferenceUpdateRequest | 

// Replace the signed-in person's notification preference.
NotificationPreferencesAPI.replaceNotificationPreference(householdId: householdId, notificationPreferenceUpdateRequest: notificationPreferenceUpdateRequest) { (response, error) in
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
 **notificationPreferenceUpdateRequest** | [**NotificationPreferenceUpdateRequest**](NotificationPreferenceUpdateRequest.md) |  | 

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateNotificationPreference**
```swift
    open class func updateNotificationPreference(householdId: Int, notificationPreferenceUpdateRequest: NotificationPreferenceUpdateRequest, completion: @escaping (_ data: NotificationPreferenceResponse?, _ error: Error?) -> Void)
```

Update the signed-in person's notification preference.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let notificationPreferenceUpdateRequest = NotificationPreferenceUpdateRequest(notificationPreference: NotificationPreferenceAttributes(enabled: false, doseDueEnabled: false, missedDoseEnabled: false, lowStockEnabled: false, privateTextEnabled: false, morningTime: "morningTime_example", afternoonTime: "afternoonTime_example", eveningTime: "eveningTime_example", nightTime: "nightTime_example")) // NotificationPreferenceUpdateRequest | 

// Update the signed-in person's notification preference.
NotificationPreferencesAPI.updateNotificationPreference(householdId: householdId, notificationPreferenceUpdateRequest: notificationPreferenceUpdateRequest) { (response, error) in
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
 **notificationPreferenceUpdateRequest** | [**NotificationPreferenceUpdateRequest**](NotificationPreferenceUpdateRequest.md) |  | 

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

