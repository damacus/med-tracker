# AccessGrantsAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createPersonAccessGrant**](AccessGrantsAPI.md#createpersonaccessgrant) | **POST** /households/{household_id}/admin/person_access_grants | Create a person access grant.
[**deletePersonAccessGrant**](AccessGrantsAPI.md#deletepersonaccessgrant) | **DELETE** /households/{household_id}/admin/person_access_grants/{id} | Revoke a person access grant.
[**listPersonAccessGrants**](AccessGrantsAPI.md#listpersonaccessgrants) | **GET** /households/{household_id}/admin/person_access_grants | List person access grants.


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
AccessGrantsAPI.createPersonAccessGrant(householdId: householdId, personAccessGrantCreateRequest: personAccessGrantCreateRequest) { (response, error) in
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
AccessGrantsAPI.deletePersonAccessGrant(householdId: householdId, id: id) { (response, error) in
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
AccessGrantsAPI.listPersonAccessGrants(householdId: householdId) { (response, error) in
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

