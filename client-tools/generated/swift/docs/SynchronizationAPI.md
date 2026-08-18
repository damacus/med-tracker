# SynchronizationAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createSyncBatch**](SynchronizationAPI.md#createsyncbatch) | **POST** /households/{household_id}/sync/batches | Apply transactional sync batch operations.
[**getMobileSnapshot**](SynchronizationAPI.md#getmobilesnapshot) | **GET** /households/{household_id}/mobile_snapshot | Read a plaintext mobile sync snapshot over authenticated transport.
[**getSyncChanges**](SynchronizationAPI.md#getsyncchanges) | **GET** /households/{household_id}/sync/changes | Read sync changes after a cursor.
[**getSyncSnapshot**](SynchronizationAPI.md#getsyncsnapshot) | **GET** /households/{household_id}/sync/snapshot | Read a portable v2 sync snapshot.


# **createSyncBatch**
```swift
    open class func createSyncBatch(householdId: Int, syncBatchRequest: SyncBatchRequest, completion: @escaping (_ data: SyncBatchResponse?, _ error: Error?) -> Void)
```

Apply transactional sync batch operations.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let syncBatchRequest = SyncBatchRequest(batch: SyncBatchRequest_batch(operations: [SyncBatchOperation(action: "action_example", resourceType: "resourceType_example", id: "id_example", ifMatch: "ifMatch_example", attributes: "TODO")])) // SyncBatchRequest | 

// Apply transactional sync batch operations.
SynchronizationAPI.createSyncBatch(householdId: householdId, syncBatchRequest: syncBatchRequest) { (response, error) in
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
 **syncBatchRequest** | [**SyncBatchRequest**](SyncBatchRequest.md) |  | 

### Return type

[**SyncBatchResponse**](SyncBatchResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getMobileSnapshot**
```swift
    open class func getMobileSnapshot(householdId: Int, completion: @escaping (_ data: PortableSnapshotResponse?, _ error: Error?) -> Void)
```

Read a plaintext mobile sync snapshot over authenticated transport.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read a plaintext mobile sync snapshot over authenticated transport.
SynchronizationAPI.getMobileSnapshot(householdId: householdId) { (response, error) in
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

[**PortableSnapshotResponse**](PortableSnapshotResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSyncChanges**
```swift
    open class func getSyncChanges(householdId: Int, cursor: Date, completion: @escaping (_ data: SyncChangesResponse?, _ error: Error?) -> Void)
```

Read sync changes after a cursor.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let cursor = Date() // Date | Return changes and tombstones at or after this timestamp.

// Read sync changes after a cursor.
SynchronizationAPI.getSyncChanges(householdId: householdId, cursor: cursor) { (response, error) in
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
 **cursor** | **Date** | Return changes and tombstones at or after this timestamp. | 

### Return type

[**SyncChangesResponse**](SyncChangesResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSyncSnapshot**
```swift
    open class func getSyncSnapshot(householdId: Int, completion: @escaping (_ data: SyncSnapshotResponse?, _ error: Error?) -> Void)
```

Read a portable v2 sync snapshot.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read a portable v2 sync snapshot.
SynchronizationAPI.getSyncSnapshot(householdId: householdId) { (response, error) in
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

[**SyncSnapshotResponse**](SyncSnapshotResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

