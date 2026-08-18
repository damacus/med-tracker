# InvitationsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createInvitation**](InvitationsAPI.md#createinvitation) | **POST** /households/{household_id}/admin/invitations | Create a household invitation.
[**deleteInvitation**](InvitationsAPI.md#deleteinvitation) | **DELETE** /households/{household_id}/admin/invitations/{id} | Revoke a household invitation.
[**listInvitations**](InvitationsAPI.md#listinvitations) | **GET** /households/{household_id}/admin/invitations | List household invitations.


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
InvitationsAPI.createInvitation(householdId: householdId, householdInvitationCreateRequest: householdInvitationCreateRequest) { (response, error) in
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
InvitationsAPI.deleteInvitation(householdId: householdId, id: id) { (response, error) in
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
InvitationsAPI.listInvitations(householdId: householdId) { (response, error) in
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

