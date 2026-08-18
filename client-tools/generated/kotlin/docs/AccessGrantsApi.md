# AccessGrantsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createPersonAccessGrant**](AccessGrantsApi.md#createPersonAccessGrant) | **POST** /households/{household_id}/admin/person_access_grants | Create a person access grant. |
| [**deletePersonAccessGrant**](AccessGrantsApi.md#deletePersonAccessGrant) | **DELETE** /households/{household_id}/admin/person_access_grants/{id} | Revoke a person access grant. |
| [**listPersonAccessGrants**](AccessGrantsApi.md#listPersonAccessGrants) | **GET** /households/{household_id}/admin/person_access_grants | List person access grants. |


<a id="createPersonAccessGrant"></a>
# **createPersonAccessGrant**
> PersonAccessGrantResponse createPersonAccessGrant(householdId, personAccessGrantCreateRequest)

Create a person access grant.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AccessGrantsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val personAccessGrantCreateRequest : PersonAccessGrantCreateRequest =  // PersonAccessGrantCreateRequest | 
try {
    val result : PersonAccessGrantResponse = apiInstance.createPersonAccessGrant(householdId, personAccessGrantCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AccessGrantsApi#createPersonAccessGrant")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccessGrantsApi#createPersonAccessGrant")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personAccessGrantCreateRequest** | [**PersonAccessGrantCreateRequest**](PersonAccessGrantCreateRequest.md)|  | |

### Return type

[**PersonAccessGrantResponse**](PersonAccessGrantResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="deletePersonAccessGrant"></a>
# **deletePersonAccessGrant**
> deletePersonAccessGrant(householdId, id)

Revoke a person access grant.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AccessGrantsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deletePersonAccessGrant(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling AccessGrantsApi#deletePersonAccessGrant")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccessGrantsApi#deletePersonAccessGrant")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int**|  | |

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listPersonAccessGrants"></a>
# **listPersonAccessGrants**
> PersonAccessGrantCollectionResponse listPersonAccessGrants(householdId)

List person access grants.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = AccessGrantsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : PersonAccessGrantCollectionResponse = apiInstance.listPersonAccessGrants(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling AccessGrantsApi#listPersonAccessGrants")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling AccessGrantsApi#listPersonAccessGrants")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**PersonAccessGrantCollectionResponse**](PersonAccessGrantCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

