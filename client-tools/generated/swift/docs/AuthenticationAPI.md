# AuthenticationAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createLoginSession**](AuthenticationAPI.md#createloginsession) | **POST** /auth/login | Exchange development or migration password credentials for API tokens.
[**deleteSession**](AuthenticationAPI.md#deletesession) | **DELETE** /auth/sessions/{id} | Revoke an API session.
[**exchangeOidcSession**](AuthenticationAPI.md#exchangeoidcsession) | **POST** /auth/oidc_exchange | Exchange a hosted OIDC ID token for an API session.
[**listHouseholds**](AuthenticationAPI.md#listhouseholds) | **GET** /auth/households | List households available to the current API credential.
[**listSessions**](AuthenticationAPI.md#listsessions) | **GET** /auth/sessions | List active API sessions for the current account.
[**logoutSession**](AuthenticationAPI.md#logoutsession) | **DELETE** /auth/logout | Revoke the current API session or app token.
[**refreshSession**](AuthenticationAPI.md#refreshsession) | **POST** /auth/refresh | Rotate an API session refresh token.


# **createLoginSession**
```swift
    open class func createLoginSession(authLoginRequest: AuthLoginRequest, completion: @escaping (_ data: AuthLoginResponse?, _ error: Error?) -> Void)
```

Exchange development or migration password credentials for API tokens.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let authLoginRequest = AuthLoginRequest(email: "email_example", password: "password_example", deviceName: "deviceName_example", householdId: 123) // AuthLoginRequest | 

// Exchange development or migration password credentials for API tokens.
AuthenticationAPI.createLoginSession(authLoginRequest: authLoginRequest) { (response, error) in
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
 **authLoginRequest** | [**AuthLoginRequest**](AuthLoginRequest.md) |  | 

### Return type

[**AuthLoginResponse**](AuthLoginResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

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
AuthenticationAPI.deleteSession(id: id) { (response, error) in
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

# **exchangeOidcSession**
```swift
    open class func exchangeOidcSession(authOidcExchangeRequest: AuthOidcExchangeRequest, completion: @escaping (_ data: AuthLoginResponse?, _ error: Error?) -> Void)
```

Exchange a hosted OIDC ID token for an API session.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let authOidcExchangeRequest = AuthOidcExchangeRequest(idToken: "idToken_example", nonce: "nonce_example", codeVerifier: "codeVerifier_example", deviceName: "deviceName_example", householdId: 123, provider: "provider_example") // AuthOidcExchangeRequest | 

// Exchange a hosted OIDC ID token for an API session.
AuthenticationAPI.exchangeOidcSession(authOidcExchangeRequest: authOidcExchangeRequest) { (response, error) in
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
 **authOidcExchangeRequest** | [**AuthOidcExchangeRequest**](AuthOidcExchangeRequest.md) |  | 

### Return type

[**AuthLoginResponse**](AuthLoginResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
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
AuthenticationAPI.listHouseholds() { (response, error) in
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
AuthenticationAPI.listSessions() { (response, error) in
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
AuthenticationAPI.logoutSession() { (response, error) in
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
AuthenticationAPI.refreshSession(authRefreshRequest: authRefreshRequest) { (response, error) in
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

