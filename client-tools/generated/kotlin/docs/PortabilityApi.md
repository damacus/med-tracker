# PortabilityApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createPortableImport**](PortabilityApi.md#createPortableImport) | **POST** /households/{household_id}/portable_imports | Apply an encrypted portable import transactionally. |
| [**dryRunPortableImport**](PortabilityApi.md#dryRunPortableImport) | **POST** /households/{household_id}/portable_imports/dry_run | Validate an encrypted portable import without writing data. |
| [**exportPortableHouseholdBundle**](PortabilityApi.md#exportPortableHouseholdBundle) | **GET** /households/{household_id}/portable_export | Export an encrypted portable household bundle. |
| [**getDataExport**](PortabilityApi.md#getDataExport) | **GET** /households/{household_id}/data_exports/{mode} | Export household data in a selected backup/profile mode. |


<a id="createPortableImport"></a>
# **createPortableImport**
> PortableImportResultResponse createPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)

Apply an encrypted portable import transactionally.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PortabilityApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
val portableImportRequest : PortableImportRequest =  // PortableImportRequest | 
try {
    val result : PortableImportResultResponse = apiInstance.createPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PortabilityApi#createPortableImport")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PortabilityApi#createPortableImport")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md)|  | |

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="dryRunPortableImport"></a>
# **dryRunPortableImport**
> PortableImportResultResponse dryRunPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)

Validate an encrypted portable import without writing data.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PortabilityApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
val portableImportRequest : PortableImportRequest =  // PortableImportRequest | 
try {
    val result : PortableImportResultResponse = apiInstance.dryRunPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PortabilityApi#dryRunPortableImport")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PortabilityApi#dryRunPortableImport")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md)|  | |

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="exportPortableHouseholdBundle"></a>
# **exportPortableHouseholdBundle**
> PortableEnvelopeResponse exportPortableHouseholdBundle(householdId, xMedTrackerPortablePassphrase)

Export an encrypted portable household bundle.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PortabilityApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
try {
    val result : PortableEnvelopeResponse = apiInstance.exportPortableHouseholdBundle(householdId, xMedTrackerPortablePassphrase)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PortabilityApi#exportPortableHouseholdBundle")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PortabilityApi#exportPortableHouseholdBundle")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | |

### Return type

[**PortableEnvelopeResponse**](PortableEnvelopeResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getDataExport"></a>
# **getDataExport**
> DataExportResponse getDataExport(householdId, mode, xMedTrackerPortablePassphrase)

Export household data in a selected backup/profile mode.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PortabilityApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val mode : kotlin.String = mode_example // kotlin.String | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Required only for the encrypted migration bundle mode.
try {
    val result : DataExportResponse = apiInstance.getDataExport(householdId, mode, xMedTrackerPortablePassphrase)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PortabilityApi#getDataExport")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PortabilityApi#getDataExport")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **mode** | **kotlin.String**|  | [enum: encrypted_migration_bundle, backup_zip, health_data_json] |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Required only for the encrypted migration bundle mode. | [optional] |

### Return type

[**DataExportResponse**](DataExportResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

