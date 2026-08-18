# MembershipsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**deleteMembership**](MembershipsApi.md#deleteMembership) | **DELETE** /households/{household_id}/admin/memberships/{id} | Revoke a household membership. |
| [**listMemberships**](MembershipsApi.md#listMemberships) | **GET** /households/{household_id}/admin/memberships | List household memberships. |
| [**replaceMembership**](MembershipsApi.md#replaceMembership) | **PUT** /households/{household_id}/admin/memberships/{id} | Replace a household membership. |
| [**updateMembership**](MembershipsApi.md#updateMembership) | **PATCH** /households/{household_id}/admin/memberships/{id} | Update a household membership. |


<a id="deleteMembership"></a>
# **deleteMembership**
> deleteMembership(householdId, id)

Revoke a household membership.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MembershipsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteMembership(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling MembershipsApi#deleteMembership")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MembershipsApi#deleteMembership")
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

<a id="listMemberships"></a>
# **listMemberships**
> HouseholdMembershipCollectionResponse listMemberships(householdId)

List household memberships.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MembershipsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : HouseholdMembershipCollectionResponse = apiInstance.listMemberships(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MembershipsApi#listMemberships")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MembershipsApi#listMemberships")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**HouseholdMembershipCollectionResponse**](HouseholdMembershipCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="replaceMembership"></a>
# **replaceMembership**
> HouseholdMembershipResponse replaceMembership(householdId, id, householdMembershipUpdateRequest)

Replace a household membership.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MembershipsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val householdMembershipUpdateRequest : HouseholdMembershipUpdateRequest =  // HouseholdMembershipUpdateRequest | 
try {
    val result : HouseholdMembershipResponse = apiInstance.replaceMembership(householdId, id, householdMembershipUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MembershipsApi#replaceMembership")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MembershipsApi#replaceMembership")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdMembershipUpdateRequest** | [**HouseholdMembershipUpdateRequest**](HouseholdMembershipUpdateRequest.md)|  | |

### Return type

[**HouseholdMembershipResponse**](HouseholdMembershipResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updateMembership"></a>
# **updateMembership**
> HouseholdMembershipResponse updateMembership(householdId, id, householdMembershipUpdateRequest)

Update a household membership.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = MembershipsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val householdMembershipUpdateRequest : HouseholdMembershipUpdateRequest =  // HouseholdMembershipUpdateRequest | 
try {
    val result : HouseholdMembershipResponse = apiInstance.updateMembership(householdId, id, householdMembershipUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling MembershipsApi#updateMembership")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling MembershipsApi#updateMembership")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdMembershipUpdateRequest** | [**HouseholdMembershipUpdateRequest**](HouseholdMembershipUpdateRequest.md)|  | |

### Return type

[**HouseholdMembershipResponse**](HouseholdMembershipResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

