# AuditLogsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**listAuditLogs**](AuditLogsApi.md#listAuditLogs) | **GET** /households/{household_id}/admin/audit_logs | List household security audit events. |


<a id="listAuditLogs"></a>
# **listAuditLogs**
> SecurityAuditEventCollectionResponse listAuditLogs(householdId)

List household security audit events.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AuditLogsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : SecurityAuditEventCollectionResponse = apiInstance.listAuditLogs(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AuditLogsApi#listAuditLogs")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AuditLogsApi#listAuditLogs")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**SecurityAuditEventCollectionResponse**](SecurityAuditEventCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

