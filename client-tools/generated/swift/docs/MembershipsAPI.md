# MembershipsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**deleteMembership**](MembershipsAPI.md#deletemembership) | **DELETE** /households/{household_id}/admin/memberships/{id} | Revoke a household membership.
[**listMemberships**](MembershipsAPI.md#listmemberships) | **GET** /households/{household_id}/admin/memberships | List household memberships.
[**replaceMembership**](MembershipsAPI.md#replacemembership) | **PUT** /households/{household_id}/admin/memberships/{id} | Replace a household membership.
[**updateMembership**](MembershipsAPI.md#updatemembership) | **PATCH** /households/{household_id}/admin/memberships/{id} | Update a household membership.


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
MembershipsAPI.deleteMembership(householdId: householdId, id: id) { (response, error) in
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
MembershipsAPI.listMemberships(householdId: householdId) { (response, error) in
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
MembershipsAPI.replaceMembership(householdId: householdId, id: id, householdMembershipUpdateRequest: householdMembershipUpdateRequest) { (response, error) in
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
MembershipsAPI.updateMembership(householdId: householdId, id: id, householdMembershipUpdateRequest: householdMembershipUpdateRequest) { (response, error) in
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

