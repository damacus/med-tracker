# DevicesAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createNativeDeviceToken**](DevicesAPI.md#createnativedevicetoken) | **POST** /households/{household_id}/native_device_tokens | Register a native device token.
[**createPushSubscription**](DevicesAPI.md#createpushsubscription) | **POST** /households/{household_id}/push_subscription | Register a web push subscription.
[**deleteNativeDeviceToken**](DevicesAPI.md#deletenativedevicetoken) | **DELETE** /households/{household_id}/native_device_tokens/{id} | Revoke a native device token.
[**deletePushSubscription**](DevicesAPI.md#deletepushsubscription) | **DELETE** /households/{household_id}/push_subscription | Revoke a web push subscription.
[**testPushSubscription**](DevicesAPI.md#testpushsubscription) | **POST** /households/{household_id}/push_subscription/test | Send a test push notification.


# **createNativeDeviceToken**
```swift
    open class func createNativeDeviceToken(householdId: Int, nativeDeviceTokenCreateRequest: NativeDeviceTokenCreateRequest, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Register a native device token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let nativeDeviceTokenCreateRequest = NativeDeviceTokenCreateRequest(nativeDeviceToken: NativeDeviceTokenAttributes(deviceToken: "deviceToken_example", platform: "platform_example")) // NativeDeviceTokenCreateRequest | 

// Register a native device token.
DevicesAPI.createNativeDeviceToken(householdId: householdId, nativeDeviceTokenCreateRequest: nativeDeviceTokenCreateRequest) { (response, error) in
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
 **nativeDeviceTokenCreateRequest** | [**NativeDeviceTokenCreateRequest**](NativeDeviceTokenCreateRequest.md) |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createPushSubscription**
```swift
    open class func createPushSubscription(householdId: Int, pushSubscriptionCreateRequest: PushSubscriptionCreateRequest, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Register a web push subscription.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let pushSubscriptionCreateRequest = PushSubscriptionCreateRequest(pushSubscription: PushSubscriptionAttributes(endpoint: "endpoint_example", keys: PushSubscriptionKeys(p256dh: "p256dh_example", auth: "auth_example"))) // PushSubscriptionCreateRequest | 

// Register a web push subscription.
DevicesAPI.createPushSubscription(householdId: householdId, pushSubscriptionCreateRequest: pushSubscriptionCreateRequest) { (response, error) in
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
 **pushSubscriptionCreateRequest** | [**PushSubscriptionCreateRequest**](PushSubscriptionCreateRequest.md) |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteNativeDeviceToken**
```swift
    open class func deleteNativeDeviceToken(householdId: Int, id: String, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke a native device token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Revoke a native device token.
DevicesAPI.deleteNativeDeviceToken(householdId: householdId, id: id) { (response, error) in
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

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deletePushSubscription**
```swift
    open class func deletePushSubscription(householdId: Int, endpoint: String, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke a web push subscription.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let endpoint = "endpoint_example" // String | 

// Revoke a web push subscription.
DevicesAPI.deletePushSubscription(householdId: householdId, endpoint: endpoint) { (response, error) in
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
 **endpoint** | **String** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **testPushSubscription**
```swift
    open class func testPushSubscription(householdId: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Send a test push notification.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Send a test push notification.
DevicesAPI.testPushSubscription(householdId: householdId) { (response, error) in
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

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

