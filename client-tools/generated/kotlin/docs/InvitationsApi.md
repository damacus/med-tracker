# InvitationsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createInvitation**](InvitationsApi.md#createInvitation) | **POST** /households/{household_id}/admin/invitations | Create a household invitation. |
| [**deleteInvitation**](InvitationsApi.md#deleteInvitation) | **DELETE** /households/{household_id}/admin/invitations/{id} | Revoke a household invitation. |
| [**listInvitations**](InvitationsApi.md#listInvitations) | **GET** /households/{household_id}/admin/invitations | List household invitations. |


<a id="createInvitation"></a>
# **createInvitation**
> HouseholdInvitationResponse createInvitation(householdId, householdInvitationCreateRequest)

Create a household invitation.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = InvitationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val householdInvitationCreateRequest : HouseholdInvitationCreateRequest =  // HouseholdInvitationCreateRequest | 
try {
    val result : HouseholdInvitationResponse = apiInstance.createInvitation(householdId, householdInvitationCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling InvitationsApi#createInvitation")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling InvitationsApi#createInvitation")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdInvitationCreateRequest** | [**HouseholdInvitationCreateRequest**](HouseholdInvitationCreateRequest.md)|  | |

### Return type

[**HouseholdInvitationResponse**](HouseholdInvitationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="deleteInvitation"></a>
# **deleteInvitation**
> deleteInvitation(householdId, id)

Revoke a household invitation.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = InvitationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteInvitation(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling InvitationsApi#deleteInvitation")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling InvitationsApi#deleteInvitation")
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

<a id="listInvitations"></a>
# **listInvitations**
> HouseholdInvitationCollectionResponse listInvitations(householdId)

List household invitations.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = InvitationsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : HouseholdInvitationCollectionResponse = apiInstance.listInvitations(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling InvitationsApi#listInvitations")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling InvitationsApi#listInvitations")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**HouseholdInvitationCollectionResponse**](HouseholdInvitationCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

