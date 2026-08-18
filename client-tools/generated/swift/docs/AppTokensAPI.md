# AppTokensAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createAppToken**](AppTokensAPI.md#createapptoken) | **POST** /households/{household_id}/admin/app_tokens | Create an API app token.
[**deleteAppToken**](AppTokensAPI.md#deleteapptoken) | **DELETE** /households/{household_id}/admin/app_tokens/{id} | Revoke an API app token.
[**listAppTokens**](AppTokensAPI.md#listapptokens) | **GET** /households/{household_id}/admin/app_tokens | List API app tokens.


# **createAppToken**
```swift
    open class func createAppToken(householdId: Int, apiAppTokenCreateRequest: ApiAppTokenCreateRequest, completion: @escaping (_ data: ApiAppTokenCreateResponse?, _ error: Error?) -> Void)
```

Create an API app token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let apiAppTokenCreateRequest = ApiAppTokenCreateRequest(apiAppToken: ApiAppTokenAttributes(name: "name_example")) // ApiAppTokenCreateRequest | 

// Create an API app token.
AppTokensAPI.createAppToken(householdId: householdId, apiAppTokenCreateRequest: apiAppTokenCreateRequest) { (response, error) in
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
 **apiAppTokenCreateRequest** | [**ApiAppTokenCreateRequest**](ApiAppTokenCreateRequest.md) |  | 

### Return type

[**ApiAppTokenCreateResponse**](ApiAppTokenCreateResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteAppToken**
```swift
    open class func deleteAppToken(householdId: Int, id: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke an API app token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 

// Revoke an API app token.
AppTokensAPI.deleteAppToken(householdId: householdId, id: id) { (response, error) in
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
 **id** | **Int** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listAppTokens**
```swift
    open class func listAppTokens(householdId: Int, completion: @escaping (_ data: ApiAppTokenCollectionResponse?, _ error: Error?) -> Void)
```

List API app tokens.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// List API app tokens.
AppTokensAPI.listAppTokens(householdId: householdId) { (response, error) in
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

[**ApiAppTokenCollectionResponse**](ApiAppTokenCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

