# PublicAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createLoginSession**](PublicAPI.md#createloginsession) | **POST** /auth/login | Exchange development or migration password credentials for API tokens.
[**exchangeOidcSession**](PublicAPI.md#exchangeoidcsession) | **POST** /auth/oidc_exchange | Exchange a hosted OIDC ID token for an API session.
[**getCapabilities**](PublicAPI.md#getcapabilities) | **GET** /capabilities | Describe supported API features, diagnostics, and first-party client-tool contracts.


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
PublicAPI.createLoginSession(authLoginRequest: authLoginRequest) { (response, error) in
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
PublicAPI.exchangeOidcSession(authOidcExchangeRequest: authOidcExchangeRequest) { (response, error) in
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

# **getCapabilities**
```swift
    open class func getCapabilities(completion: @escaping (_ data: CapabilitiesResponse?, _ error: Error?) -> Void)
```

Describe supported API features, diagnostics, and first-party client-tool contracts.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI


// Describe supported API features, diagnostics, and first-party client-tool contracts.
PublicAPI.getCapabilities() { (response, error) in
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

[**CapabilitiesResponse**](CapabilitiesResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

