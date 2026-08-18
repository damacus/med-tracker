# HouseholdAdministrationAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createAppToken**](HouseholdAdministrationAPI.md#createapptoken) | **POST** /households/{household_id}/admin/app_tokens | Create an API app token.
[**createInvitation**](HouseholdAdministrationAPI.md#createinvitation) | **POST** /households/{household_id}/admin/invitations | Create a household invitation.
[**createPersonAccessGrant**](HouseholdAdministrationAPI.md#createpersonaccessgrant) | **POST** /households/{household_id}/admin/person_access_grants | Create a person access grant.
[**deleteAppToken**](HouseholdAdministrationAPI.md#deleteapptoken) | **DELETE** /households/{household_id}/admin/app_tokens/{id} | Revoke an API app token.
[**deleteInvitation**](HouseholdAdministrationAPI.md#deleteinvitation) | **DELETE** /households/{household_id}/admin/invitations/{id} | Revoke a household invitation.
[**deleteMembership**](HouseholdAdministrationAPI.md#deletemembership) | **DELETE** /households/{household_id}/admin/memberships/{id} | Revoke a household membership.
[**deletePersonAccessGrant**](HouseholdAdministrationAPI.md#deletepersonaccessgrant) | **DELETE** /households/{household_id}/admin/person_access_grants/{id} | Revoke a person access grant.
[**getHouseholdAdminSettings**](HouseholdAdministrationAPI.md#gethouseholdadminsettings) | **GET** /households/{household_id}/admin/settings | Read household administration settings.
[**listAppTokens**](HouseholdAdministrationAPI.md#listapptokens) | **GET** /households/{household_id}/admin/app_tokens | List API app tokens.
[**listAuditLogs**](HouseholdAdministrationAPI.md#listauditlogs) | **GET** /households/{household_id}/admin/audit_logs | List household security audit events.
[**listInvitations**](HouseholdAdministrationAPI.md#listinvitations) | **GET** /households/{household_id}/admin/invitations | List household invitations.
[**listMemberships**](HouseholdAdministrationAPI.md#listmemberships) | **GET** /households/{household_id}/admin/memberships | List household memberships.
[**listPersonAccessGrants**](HouseholdAdministrationAPI.md#listpersonaccessgrants) | **GET** /households/{household_id}/admin/person_access_grants | List person access grants.
[**replaceHouseholdAdminSettings**](HouseholdAdministrationAPI.md#replacehouseholdadminsettings) | **PUT** /households/{household_id}/admin/settings | Replace household administration settings.
[**replaceMembership**](HouseholdAdministrationAPI.md#replacemembership) | **PUT** /households/{household_id}/admin/memberships/{id} | Replace a household membership.
[**updateHouseholdAdminSettings**](HouseholdAdministrationAPI.md#updatehouseholdadminsettings) | **PATCH** /households/{household_id}/admin/settings | Update household administration settings.
[**updateMembership**](HouseholdAdministrationAPI.md#updatemembership) | **PATCH** /households/{household_id}/admin/memberships/{id} | Update a household membership.


# **createAppToken**
```swift
    open class func createAppToken(householdId: Int, apiAppTokenCreateRequest: ApiAppTokenCreateRequest, completion: @escaping (_ data: ApiAppTokenCreateResponse?, _ error: Error?) -> Void)
```

Create an API app token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let apiAppTokenCreateRequest = ApiAppTokenCreateRequest(apiAppToken: ApiAppTokenAttributes(name: "name_example")) // ApiAppTokenCreateRequest | 

// Create an API app token.
HouseholdAdministrationAPI.createAppToken(householdId: householdId, apiAppTokenCreateRequest: apiAppTokenCreateRequest) { (response, error) in
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
 **apiAppTokenCreateRequest** | [**ApiAppTokenCreateRequest**](ApiAppTokenCreateRequest.md) |  | 

### Return type

[**ApiAppTokenCreateResponse**](ApiAppTokenCreateResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createInvitation**
```swift
    open class func createInvitation(householdId: Int, householdInvitationCreateRequest: HouseholdInvitationCreateRequest, completion: @escaping (_ data: HouseholdInvitationResponse?, _ error: Error?) -> Void)
```

Create a household invitation.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let householdInvitationCreateRequest = HouseholdInvitationCreateRequest(householdInvitation: HouseholdInvitationAttributes(email: "email_example", membershipRole: "membershipRole_example")) // HouseholdInvitationCreateRequest | 

// Create a household invitation.
HouseholdAdministrationAPI.createInvitation(householdId: householdId, householdInvitationCreateRequest: householdInvitationCreateRequest) { (response, error) in
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
 **householdInvitationCreateRequest** | [**HouseholdInvitationCreateRequest**](HouseholdInvitationCreateRequest.md) |  | 

### Return type

[**HouseholdInvitationResponse**](HouseholdInvitationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createPersonAccessGrant**
```swift
    open class func createPersonAccessGrant(householdId: Int, personAccessGrantCreateRequest: PersonAccessGrantCreateRequest, completion: @escaping (_ data: PersonAccessGrantResponse?, _ error: Error?) -> Void)
```

Create a person access grant.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let personAccessGrantCreateRequest = PersonAccessGrantCreateRequest(personAccessGrant: PersonAccessGrantAttributes(householdMembershipId: 123, personId: 123, accessLevel: "accessLevel_example", relationshipType: "relationshipType_example", expiresAt: Date())) // PersonAccessGrantCreateRequest | 

// Create a person access grant.
HouseholdAdministrationAPI.createPersonAccessGrant(householdId: householdId, personAccessGrantCreateRequest: personAccessGrantCreateRequest) { (response, error) in
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
 **personAccessGrantCreateRequest** | [**PersonAccessGrantCreateRequest**](PersonAccessGrantCreateRequest.md) |  | 

### Return type

[**PersonAccessGrantResponse**](PersonAccessGrantResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteAppToken**
```swift
    open class func deleteAppToken(householdId: Int, id: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke an API app token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 

// Revoke an API app token.
HouseholdAdministrationAPI.deleteAppToken(householdId: householdId, id: id) { (response, error) in
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
 **id** | **Int** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteInvitation**
```swift
    open class func deleteInvitation(householdId: Int, id: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke a household invitation.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 

// Revoke a household invitation.
HouseholdAdministrationAPI.deleteInvitation(householdId: householdId, id: id) { (response, error) in
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
 **id** | **Int** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteMembership**
```swift
    open class func deleteMembership(householdId: Int, id: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke a household membership.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 

// Revoke a household membership.
HouseholdAdministrationAPI.deleteMembership(householdId: householdId, id: id) { (response, error) in
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
 **id** | **Int** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deletePersonAccessGrant**
```swift
    open class func deletePersonAccessGrant(householdId: Int, id: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke a person access grant.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 

// Revoke a person access grant.
HouseholdAdministrationAPI.deletePersonAccessGrant(householdId: householdId, id: id) { (response, error) in
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
 **id** | **Int** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getHouseholdAdminSettings**
```swift
    open class func getHouseholdAdminSettings(householdId: Int, completion: @escaping (_ data: HouseholdAdminSettingsResponse?, _ error: Error?) -> Void)
```

Read household administration settings.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read household administration settings.
HouseholdAdministrationAPI.getHouseholdAdminSettings(householdId: householdId) { (response, error) in
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

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listAppTokens**
```swift
    open class func listAppTokens(householdId: Int, completion: @escaping (_ data: ApiAppTokenCollectionResponse?, _ error: Error?) -> Void)
```

List API app tokens.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// List API app tokens.
HouseholdAdministrationAPI.listAppTokens(householdId: householdId) { (response, error) in
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

[**ApiAppTokenCollectionResponse**](ApiAppTokenCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

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
HouseholdAdministrationAPI.listAuditLogs(householdId: householdId) { (response, error) in
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

# **listInvitations**
```swift
    open class func listInvitations(householdId: Int, completion: @escaping (_ data: HouseholdInvitationCollectionResponse?, _ error: Error?) -> Void)
```

List household invitations.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// List household invitations.
HouseholdAdministrationAPI.listInvitations(householdId: householdId) { (response, error) in
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

[**HouseholdInvitationCollectionResponse**](HouseholdInvitationCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listMemberships**
```swift
    open class func listMemberships(householdId: Int, completion: @escaping (_ data: HouseholdMembershipCollectionResponse?, _ error: Error?) -> Void)
```

List household memberships.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// List household memberships.
HouseholdAdministrationAPI.listMemberships(householdId: householdId) { (response, error) in
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

[**HouseholdMembershipCollectionResponse**](HouseholdMembershipCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPersonAccessGrants**
```swift
    open class func listPersonAccessGrants(householdId: Int, completion: @escaping (_ data: PersonAccessGrantCollectionResponse?, _ error: Error?) -> Void)
```

List person access grants.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// List person access grants.
HouseholdAdministrationAPI.listPersonAccessGrants(householdId: householdId) { (response, error) in
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

[**PersonAccessGrantCollectionResponse**](PersonAccessGrantCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceHouseholdAdminSettings**
```swift
    open class func replaceHouseholdAdminSettings(householdId: Int, householdAdminSettingsUpdateRequest: HouseholdAdminSettingsUpdateRequest, completion: @escaping (_ data: HouseholdAdminSettingsResponse?, _ error: Error?) -> Void)
```

Replace household administration settings.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let householdAdminSettingsUpdateRequest = HouseholdAdminSettingsUpdateRequest(household: HouseholdAdminSettingsAttributes(name: "name_example", timezone: "timezone_example", subscriptionPlan: "subscriptionPlan_example")) // HouseholdAdminSettingsUpdateRequest | 

// Replace household administration settings.
HouseholdAdministrationAPI.replaceHouseholdAdminSettings(householdId: householdId, householdAdminSettingsUpdateRequest: householdAdminSettingsUpdateRequest) { (response, error) in
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
 **householdAdminSettingsUpdateRequest** | [**HouseholdAdminSettingsUpdateRequest**](HouseholdAdminSettingsUpdateRequest.md) |  | 

### Return type

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceMembership**
```swift
    open class func replaceMembership(householdId: Int, id: Int, householdMembershipUpdateRequest: HouseholdMembershipUpdateRequest, completion: @escaping (_ data: HouseholdMembershipResponse?, _ error: Error?) -> Void)
```

Replace a household membership.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 
let householdMembershipUpdateRequest = HouseholdMembershipUpdateRequest(householdMembership: HouseholdMembershipAttributes(role: "role_example", status: "status_example", personId: 123)) // HouseholdMembershipUpdateRequest | 

// Replace a household membership.
HouseholdAdministrationAPI.replaceMembership(householdId: householdId, id: id, householdMembershipUpdateRequest: householdMembershipUpdateRequest) { (response, error) in
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
 **id** | **Int** |  | 
 **householdMembershipUpdateRequest** | [**HouseholdMembershipUpdateRequest**](HouseholdMembershipUpdateRequest.md) |  | 

### Return type

[**HouseholdMembershipResponse**](HouseholdMembershipResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateHouseholdAdminSettings**
```swift
    open class func updateHouseholdAdminSettings(householdId: Int, householdAdminSettingsUpdateRequest: HouseholdAdminSettingsUpdateRequest, completion: @escaping (_ data: HouseholdAdminSettingsResponse?, _ error: Error?) -> Void)
```

Update household administration settings.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let householdAdminSettingsUpdateRequest = HouseholdAdminSettingsUpdateRequest(household: HouseholdAdminSettingsAttributes(name: "name_example", timezone: "timezone_example", subscriptionPlan: "subscriptionPlan_example")) // HouseholdAdminSettingsUpdateRequest | 

// Update household administration settings.
HouseholdAdministrationAPI.updateHouseholdAdminSettings(householdId: householdId, householdAdminSettingsUpdateRequest: householdAdminSettingsUpdateRequest) { (response, error) in
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
 **householdAdminSettingsUpdateRequest** | [**HouseholdAdminSettingsUpdateRequest**](HouseholdAdminSettingsUpdateRequest.md) |  | 

### Return type

[**HouseholdAdminSettingsResponse**](HouseholdAdminSettingsResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateMembership**
```swift
    open class func updateMembership(householdId: Int, id: Int, householdMembershipUpdateRequest: HouseholdMembershipUpdateRequest, completion: @escaping (_ data: HouseholdMembershipResponse?, _ error: Error?) -> Void)
```

Update a household membership.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 
let householdMembershipUpdateRequest = HouseholdMembershipUpdateRequest(householdMembership: HouseholdMembershipAttributes(role: "role_example", status: "status_example", personId: 123)) // HouseholdMembershipUpdateRequest | 

// Update a household membership.
HouseholdAdministrationAPI.updateMembership(householdId: householdId, id: id, householdMembershipUpdateRequest: householdMembershipUpdateRequest) { (response, error) in
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
 **id** | **Int** |  | 
 **householdMembershipUpdateRequest** | [**HouseholdMembershipUpdateRequest**](HouseholdMembershipUpdateRequest.md) |  | 

### Return type

[**HouseholdMembershipResponse**](HouseholdMembershipResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

