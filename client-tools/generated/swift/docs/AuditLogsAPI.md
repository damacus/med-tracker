# AuditLogsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**listAuditLogs**](AuditLogsAPI.md#listauditlogs) | **GET** /households/{household_id}/admin/audit_logs | List household security audit events.


# **listAuditLogs**
```swift
    open class func listAuditLogs(householdId: Int, completion: @escaping (_ data: SecurityAuditEventCollectionResponse?, _ error: Error?) -> Void)
```

List household security audit events.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// List household security audit events.
AuditLogsAPI.listAuditLogs(householdId: householdId) { (response, error) in
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

[**SecurityAuditEventCollectionResponse**](SecurityAuditEventCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

