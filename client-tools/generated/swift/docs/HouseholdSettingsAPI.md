# HouseholdSettingsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getHouseholdAdminSettings**](HouseholdSettingsAPI.md#gethouseholdadminsettings) | **GET** /households/{household_id}/admin/settings | Read household administration settings.
[**replaceHouseholdAdminSettings**](HouseholdSettingsAPI.md#replacehouseholdadminsettings) | **PUT** /households/{household_id}/admin/settings | Replace household administration settings.
[**updateHouseholdAdminSettings**](HouseholdSettingsAPI.md#updatehouseholdadminsettings) | **PATCH** /households/{household_id}/admin/settings | Update household administration settings.


# **getHouseholdAdminSettings**
```swift
    open class func getHouseholdAdminSettings(householdId: Int, completion: @escaping (_ data: HouseholdAdminSettingsResponse?, _ error: Error?) -> Void)
```

Read household administration settings.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read household administration settings.
HouseholdSettingsAPI.getHouseholdAdminSettings(householdId: householdId) { (response, error) in
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

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceHouseholdAdminSettings**
```swift
    open class func replaceHouseholdAdminSettings(householdId: Int, householdAdminSettingsUpdateRequest: HouseholdAdminSettingsUpdateRequest, completion: @escaping (_ data: HouseholdAdminSettingsResponse?, _ error: Error?) -> Void)
```

Replace household administration settings.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let householdAdminSettingsUpdateRequest = HouseholdAdminSettingsUpdateRequest(household: HouseholdAdminSettingsAttributes(name: "name_example", timezone: "timezone_example", subscriptionPlan: "subscriptionPlan_example")) // HouseholdAdminSettingsUpdateRequest | 

// Replace household administration settings.
HouseholdSettingsAPI.replaceHouseholdAdminSettings(householdId: householdId, householdAdminSettingsUpdateRequest: householdAdminSettingsUpdateRequest) { (response, error) in
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
 **householdAdminSettingsUpdateRequest** | [**HouseholdAdminSettingsUpdateRequest**](HouseholdAdminSettingsUpdateRequest.md) |  | 

### Return type

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateHouseholdAdminSettings**
```swift
    open class func updateHouseholdAdminSettings(householdId: Int, householdAdminSettingsUpdateRequest: HouseholdAdminSettingsUpdateRequest, completion: @escaping (_ data: HouseholdAdminSettingsResponse?, _ error: Error?) -> Void)
```

Update household administration settings.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let householdAdminSettingsUpdateRequest = HouseholdAdminSettingsUpdateRequest(household: HouseholdAdminSettingsAttributes(name: "name_example", timezone: "timezone_example", subscriptionPlan: "subscriptionPlan_example")) // HouseholdAdminSettingsUpdateRequest | 

// Update household administration settings.
HouseholdSettingsAPI.updateHouseholdAdminSettings(householdId: householdId, householdAdminSettingsUpdateRequest: householdAdminSettingsUpdateRequest) { (response, error) in
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
 **householdAdminSettingsUpdateRequest** | [**HouseholdAdminSettingsUpdateRequest**](HouseholdAdminSettingsUpdateRequest.md) |  | 

### Return type

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

