# AccountAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**deleteSession**](AccountAPI.md#deletesession) | **DELETE** /auth/sessions/{id} | Revoke an API session.
[**listHouseholds**](AccountAPI.md#listhouseholds) | **GET** /auth/households | List households available to the current API credential.
[**listSessions**](AccountAPI.md#listsessions) | **GET** /auth/sessions | List active API sessions for the current account.
[**logoutSession**](AccountAPI.md#logoutsession) | **DELETE** /auth/logout | Revoke the current API session or app token.
[**refreshSession**](AccountAPI.md#refreshsession) | **POST** /auth/refresh | Rotate an API session refresh token.


# **deleteSession**
```swift
    open class func deleteSession(id: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke an API session.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let id = 987 // Int | 

// Revoke an API session.
AccountAPI.deleteSession(id: id) { (response, error) in
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
 **id** | **Int** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listHouseholds**
```swift
    open class func listHouseholds(completion: @escaping (_ data: AuthHouseholdCollectionResponse?, _ error: Error?) -> Void)
```

List households available to the current API credential.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI


// List households available to the current API credential.
AccountAPI.listHouseholds() { (response, error) in
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
This endpoint does not need any parameter.

### Return type

[**AuthHouseholdCollectionResponse**](AuthHouseholdCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listSessions**
```swift
    open class func listSessions(completion: @escaping (_ data: AuthSessionCollectionResponse?, _ error: Error?) -> Void)
```

List active API sessions for the current account.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI


// List active API sessions for the current account.
AccountAPI.listSessions() { (response, error) in
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
This endpoint does not need any parameter.

### Return type

[**AuthSessionCollectionResponse**](AuthSessionCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **logoutSession**
```swift
    open class func logoutSession(completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke the current API session or app token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI


// Revoke the current API session or app token.
AccountAPI.logoutSession() { (response, error) in
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
This endpoint does not need any parameter.

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **refreshSession**
```swift
    open class func refreshSession(authRefreshRequest: AuthRefreshRequest, completion: @escaping (_ data: AuthRefreshResponse?, _ error: Error?) -> Void)
```

Rotate an API session refresh token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let authRefreshRequest = AuthRefreshRequest(refreshToken: "refreshToken_example") // AuthRefreshRequest | 

// Rotate an API session refresh token.
AccountAPI.refreshSession(authRefreshRequest: authRefreshRequest) { (response, error) in
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
 **authRefreshRequest** | [**AuthRefreshRequest**](AuthRefreshRequest.md) |  | 

### Return type

[**AuthRefreshResponse**](AuthRefreshResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

