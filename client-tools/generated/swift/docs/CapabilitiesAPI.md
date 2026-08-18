# CapabilitiesAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getCapabilities**](CapabilitiesAPI.md#getcapabilities) | **GET** /capabilities | Describe supported API features, diagnostics, and first-party client-tool contracts.


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
CapabilitiesAPI.getCapabilities() { (response, error) in
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

