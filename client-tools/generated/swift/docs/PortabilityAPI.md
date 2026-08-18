# PortabilityAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createPortableImport**](PortabilityAPI.md#createportableimport) | **POST** /households/{household_id}/portable_imports | Apply an encrypted portable import transactionally.
[**dryRunPortableImport**](PortabilityAPI.md#dryrunportableimport) | **POST** /households/{household_id}/portable_imports/dry_run | Validate an encrypted portable import without writing data.
[**exportPortableHouseholdBundle**](PortabilityAPI.md#exportportablehouseholdbundle) | **GET** /households/{household_id}/portable_export | Export an encrypted portable household bundle.
[**getDataExport**](PortabilityAPI.md#getdataexport) | **GET** /households/{household_id}/data_exports/{mode} | Export household data in a selected backup/profile mode.


# **createPortableImport**
```swift
    open class func createPortableImport(householdId: Int, xMedTrackerPortablePassphrase: String, portableImportRequest: PortableImportRequest, completion: @escaping (_ data: PortableImportResultResponse?, _ error: Error?) -> Void)
```

Apply an encrypted portable import transactionally.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
let portableImportRequest = PortableImportRequest(bundle: PortableEnvelope(format: "format_example", encryptedAt: Date(), cipher: "cipher_example", kdf: "kdf_example", salt: "salt_example", checksum: "checksum_example", ciphertext: "ciphertext_example")) // PortableImportRequest | 

// Apply an encrypted portable import transactionally.
PortabilityAPI.createPortableImport(householdId: householdId, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase, portableImportRequest: portableImportRequest) { (response, error) in
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
 **xMedTrackerPortablePassphrase** | **String** | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | 
 **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md) |  | 

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **dryRunPortableImport**
```swift
    open class func dryRunPortableImport(householdId: Int, xMedTrackerPortablePassphrase: String, portableImportRequest: PortableImportRequest, completion: @escaping (_ data: PortableImportResultResponse?, _ error: Error?) -> Void)
```

Validate an encrypted portable import without writing data.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
let portableImportRequest = PortableImportRequest(bundle: PortableEnvelope(format: "format_example", encryptedAt: Date(), cipher: "cipher_example", kdf: "kdf_example", salt: "salt_example", checksum: "checksum_example", ciphertext: "ciphertext_example")) // PortableImportRequest | 

// Validate an encrypted portable import without writing data.
PortabilityAPI.dryRunPortableImport(householdId: householdId, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase, portableImportRequest: portableImportRequest) { (response, error) in
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
 **xMedTrackerPortablePassphrase** | **String** | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | 
 **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md) |  | 

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **exportPortableHouseholdBundle**
```swift
    open class func exportPortableHouseholdBundle(householdId: Int, xMedTrackerPortablePassphrase: String, completion: @escaping (_ data: PortableEnvelopeResponse?, _ error: Error?) -> Void)
```

Export an encrypted portable household bundle.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.

// Export an encrypted portable household bundle.
PortabilityAPI.exportPortableHouseholdBundle(householdId: householdId, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase) { (response, error) in
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
 **xMedTrackerPortablePassphrase** | **String** | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | 

### Return type

[**PortableEnvelopeResponse**](PortableEnvelopeResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getDataExport**
```swift
    open class func getDataExport(householdId: Int, mode: Mode_getDataExport, xMedTrackerPortablePassphrase: String? = nil, completion: @escaping (_ data: DataExportResponse?, _ error: Error?) -> Void)
```

Export household data in a selected backup/profile mode.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let mode = "mode_example" // String | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Required only for the encrypted migration bundle mode. (optional)

// Export household data in a selected backup/profile mode.
PortabilityAPI.getDataExport(householdId: householdId, mode: mode, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase) { (response, error) in
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
 **mode** | **String** |  | 
 **xMedTrackerPortablePassphrase** | **String** | Required only for the encrypted migration bundle mode. | [optional] 

### Return type

[**DataExportResponse**](DataExportResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

