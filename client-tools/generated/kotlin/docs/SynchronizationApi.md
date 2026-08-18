# SynchronizationApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createSyncBatch**](SynchronizationApi.md#createSyncBatch) | **POST** /households/{household_id}/sync/batches | Apply transactional sync batch operations. |
| [**getMobileSnapshot**](SynchronizationApi.md#getMobileSnapshot) | **GET** /households/{household_id}/mobile_snapshot | Read a plaintext mobile sync snapshot over authenticated transport. |
| [**getSyncChanges**](SynchronizationApi.md#getSyncChanges) | **GET** /households/{household_id}/sync/changes | Read sync changes after a cursor. |
| [**getSyncSnapshot**](SynchronizationApi.md#getSyncSnapshot) | **GET** /households/{household_id}/sync/snapshot | Read a portable v2 sync snapshot. |


<a id="createSyncBatch"></a>
# **createSyncBatch**
> SyncBatchResponse createSyncBatch(householdId, syncBatchRequest)

Apply transactional sync batch operations.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SynchronizationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val syncBatchRequest : SyncBatchRequest =  // SyncBatchRequest | 
try {
    val result : SyncBatchResponse = apiInstance.createSyncBatch(householdId, syncBatchRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SynchronizationApi#createSyncBatch")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SynchronizationApi#createSyncBatch")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **syncBatchRequest** | [**SyncBatchRequest**](SyncBatchRequest.md)|  | |

### Return type

[**SyncBatchResponse**](SyncBatchResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="getMobileSnapshot"></a>
# **getMobileSnapshot**
> PortableSnapshotResponse getMobileSnapshot(householdId)

Read a plaintext mobile sync snapshot over authenticated transport.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SynchronizationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : PortableSnapshotResponse = apiInstance.getMobileSnapshot(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SynchronizationApi#getMobileSnapshot")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SynchronizationApi#getMobileSnapshot")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**PortableSnapshotResponse**](PortableSnapshotResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getSyncChanges"></a>
# **getSyncChanges**
> SyncChangesResponse getSyncChanges(householdId, cursor)

Read sync changes after a cursor.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SynchronizationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val cursor : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | Return changes and tombstones at or after this timestamp.
try {
    val result : SyncChangesResponse = apiInstance.getSyncChanges(householdId, cursor)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SynchronizationApi#getSyncChanges")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SynchronizationApi#getSyncChanges")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **cursor** | **java.time.OffsetDateTime**| Return changes and tombstones at or after this timestamp. | |

### Return type

[**SyncChangesResponse**](SyncChangesResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getSyncSnapshot"></a>
# **getSyncSnapshot**
> SyncSnapshotResponse getSyncSnapshot(householdId)

Read a portable v2 sync snapshot.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = SynchronizationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : SyncSnapshotResponse = apiInstance.getSyncSnapshot(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling SynchronizationApi#getSyncSnapshot")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling SynchronizationApi#getSyncSnapshot")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**SyncSnapshotResponse**](SyncSnapshotResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

