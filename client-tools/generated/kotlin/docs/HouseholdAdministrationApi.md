# HouseholdAdministrationApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createAppToken**](HouseholdAdministrationApi.md#createAppToken) | **POST** /households/{household_id}/admin/app_tokens | Create an API app token. |
| [**createInvitation**](HouseholdAdministrationApi.md#createInvitation) | **POST** /households/{household_id}/admin/invitations | Create a household invitation. |
| [**createPersonAccessGrant**](HouseholdAdministrationApi.md#createPersonAccessGrant) | **POST** /households/{household_id}/admin/person_access_grants | Create a person access grant. |
| [**deleteAppToken**](HouseholdAdministrationApi.md#deleteAppToken) | **DELETE** /households/{household_id}/admin/app_tokens/{id} | Revoke an API app token. |
| [**deleteInvitation**](HouseholdAdministrationApi.md#deleteInvitation) | **DELETE** /households/{household_id}/admin/invitations/{id} | Revoke a household invitation. |
| [**deleteMembership**](HouseholdAdministrationApi.md#deleteMembership) | **DELETE** /households/{household_id}/admin/memberships/{id} | Revoke a household membership. |
| [**deletePersonAccessGrant**](HouseholdAdministrationApi.md#deletePersonAccessGrant) | **DELETE** /households/{household_id}/admin/person_access_grants/{id} | Revoke a person access grant. |
| [**getHouseholdAdminSettings**](HouseholdAdministrationApi.md#getHouseholdAdminSettings) | **GET** /households/{household_id}/admin/settings | Read household administration settings. |
| [**listAppTokens**](HouseholdAdministrationApi.md#listAppTokens) | **GET** /households/{household_id}/admin/app_tokens | List API app tokens. |
| [**listAuditLogs**](HouseholdAdministrationApi.md#listAuditLogs) | **GET** /households/{household_id}/admin/audit_logs | List household security audit events. |
| [**listInvitations**](HouseholdAdministrationApi.md#listInvitations) | **GET** /households/{household_id}/admin/invitations | List household invitations. |
| [**listMemberships**](HouseholdAdministrationApi.md#listMemberships) | **GET** /households/{household_id}/admin/memberships | List household memberships. |
| [**listPersonAccessGrants**](HouseholdAdministrationApi.md#listPersonAccessGrants) | **GET** /households/{household_id}/admin/person_access_grants | List person access grants. |
| [**replaceHouseholdAdminSettings**](HouseholdAdministrationApi.md#replaceHouseholdAdminSettings) | **PUT** /households/{household_id}/admin/settings | Replace household administration settings. |
| [**replaceMembership**](HouseholdAdministrationApi.md#replaceMembership) | **PUT** /households/{household_id}/admin/memberships/{id} | Replace a household membership. |
| [**updateHouseholdAdminSettings**](HouseholdAdministrationApi.md#updateHouseholdAdminSettings) | **PATCH** /households/{household_id}/admin/settings | Update household administration settings. |
| [**updateMembership**](HouseholdAdministrationApi.md#updateMembership) | **PATCH** /households/{household_id}/admin/memberships/{id} | Update a household membership. |


<a id="createAppToken"></a>
# **createAppToken**
> ApiAppTokenCreateResponse createAppToken(householdId, apiAppTokenCreateRequest)

Create an API app token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val apiAppTokenCreateRequest : ApiAppTokenCreateRequest =  // ApiAppTokenCreateRequest | 
try {
    val result : ApiAppTokenCreateResponse = apiInstance.createAppToken(householdId, apiAppTokenCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#createAppToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#createAppToken")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **apiAppTokenCreateRequest** | [**ApiAppTokenCreateRequest**](ApiAppTokenCreateRequest.md)|  | |

### Return type

[**ApiAppTokenCreateResponse**](ApiAppTokenCreateResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createInvitation"></a>
# **createInvitation**
> HouseholdInvitationResponse createInvitation(householdId, householdInvitationCreateRequest)

Create a household invitation.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val householdInvitationCreateRequest : HouseholdInvitationCreateRequest =  // HouseholdInvitationCreateRequest | 
try {
    val result : HouseholdInvitationResponse = apiInstance.createInvitation(householdId, householdInvitationCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#createInvitation")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#createInvitation")
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

<a id="createPersonAccessGrant"></a>
# **createPersonAccessGrant**
> PersonAccessGrantResponse createPersonAccessGrant(householdId, personAccessGrantCreateRequest)

Create a person access grant.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val personAccessGrantCreateRequest : PersonAccessGrantCreateRequest =  // PersonAccessGrantCreateRequest | 
try {
    val result : PersonAccessGrantResponse = apiInstance.createPersonAccessGrant(householdId, personAccessGrantCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#createPersonAccessGrant")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#createPersonAccessGrant")
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

<a id="deleteAppToken"></a>
# **deleteAppToken**
> deleteAppToken(householdId, id)

Revoke an API app token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteAppToken(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#deleteAppToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#deleteAppToken")
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

<a id="deleteInvitation"></a>
# **deleteInvitation**
> deleteInvitation(householdId, id)

Revoke a household invitation.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteInvitation(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#deleteInvitation")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#deleteInvitation")
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

<a id="deleteMembership"></a>
# **deleteMembership**
> deleteMembership(householdId, id)

Revoke a household membership.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deleteMembership(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#deleteMembership")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#deleteMembership")
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

<a id="deletePersonAccessGrant"></a>
# **deletePersonAccessGrant**
> deletePersonAccessGrant(householdId, id)

Revoke a person access grant.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.deletePersonAccessGrant(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#deletePersonAccessGrant")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#deletePersonAccessGrant")
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

<a id="getHouseholdAdminSettings"></a>
# **getHouseholdAdminSettings**
> HouseholdAdminSettingsResponse getHouseholdAdminSettings(householdId)

Read household administration settings.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : HouseholdAdminSettingsResponse = apiInstance.getHouseholdAdminSettings(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#getHouseholdAdminSettings")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#getHouseholdAdminSettings")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listAppTokens"></a>
# **listAppTokens**
> ApiAppTokenCollectionResponse listAppTokens(householdId)

List API app tokens.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : ApiAppTokenCollectionResponse = apiInstance.listAppTokens(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#listAppTokens")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#listAppTokens")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**ApiAppTokenCollectionResponse**](ApiAppTokenCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listAuditLogs"></a>
# **listAuditLogs**
> SecurityAuditEventCollectionResponse listAuditLogs(householdId)

List household security audit events.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : SecurityAuditEventCollectionResponse = apiInstance.listAuditLogs(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#listAuditLogs")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#listAuditLogs")
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

<a id="listInvitations"></a>
# **listInvitations**
> HouseholdInvitationCollectionResponse listInvitations(householdId)

List household invitations.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : HouseholdInvitationCollectionResponse = apiInstance.listInvitations(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#listInvitations")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#listInvitations")
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

<a id="listMemberships"></a>
# **listMemberships**
> HouseholdMembershipCollectionResponse listMemberships(householdId)

List household memberships.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : HouseholdMembershipCollectionResponse = apiInstance.listMemberships(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#listMemberships")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#listMemberships")
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

<a id="listPersonAccessGrants"></a>
# **listPersonAccessGrants**
> PersonAccessGrantCollectionResponse listPersonAccessGrants(householdId)

List person access grants.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : PersonAccessGrantCollectionResponse = apiInstance.listPersonAccessGrants(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#listPersonAccessGrants")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#listPersonAccessGrants")
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

<a id="replaceHouseholdAdminSettings"></a>
# **replaceHouseholdAdminSettings**
> HouseholdAdminSettingsResponse replaceHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)

Replace household administration settings.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val householdAdminSettingsUpdateRequest : HouseholdAdminSettingsUpdateRequest =  // HouseholdAdminSettingsUpdateRequest | 
try {
    val result : HouseholdAdminSettingsResponse = apiInstance.replaceHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#replaceHouseholdAdminSettings")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#replaceHouseholdAdminSettings")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdAdminSettingsUpdateRequest** | [**HouseholdAdminSettingsUpdateRequest**](HouseholdAdminSettingsUpdateRequest.md)|  | |

### Return type

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
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

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val householdMembershipUpdateRequest : HouseholdMembershipUpdateRequest =  // HouseholdMembershipUpdateRequest | 
try {
    val result : HouseholdMembershipResponse = apiInstance.replaceMembership(householdId, id, householdMembershipUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#replaceMembership")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#replaceMembership")
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

<a id="updateHouseholdAdminSettings"></a>
# **updateHouseholdAdminSettings**
> HouseholdAdminSettingsResponse updateHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)

Update household administration settings.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val householdAdminSettingsUpdateRequest : HouseholdAdminSettingsUpdateRequest =  // HouseholdAdminSettingsUpdateRequest | 
try {
    val result : HouseholdAdminSettingsResponse = apiInstance.updateHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#updateHouseholdAdminSettings")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#updateHouseholdAdminSettings")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdAdminSettingsUpdateRequest** | [**HouseholdAdminSettingsUpdateRequest**](HouseholdAdminSettingsUpdateRequest.md)|  | |

### Return type

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

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

val apiInstance = HouseholdAdministrationApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val householdMembershipUpdateRequest : HouseholdMembershipUpdateRequest =  // HouseholdMembershipUpdateRequest | 
try {
    val result : HouseholdMembershipResponse = apiInstance.updateMembership(householdId, id, householdMembershipUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdAdministrationApi#updateMembership")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdAdministrationApi#updateMembership")
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

