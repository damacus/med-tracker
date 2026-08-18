# PeopleAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createPerson**](PeopleAPI.md#createperson) | **POST** /households/{household_id}/people | Create a person in the household.
[**getCurrentProfile**](PeopleAPI.md#getcurrentprofile) | **GET** /households/{household_id}/me | Read the current account and person profile for a household session.
[**getPerson**](PeopleAPI.md#getperson) | **GET** /households/{household_id}/people/{id} | Read a person.
[**listPeople**](PeopleAPI.md#listpeople) | **GET** /households/{household_id}/people | List visible people in the household.
[**updatePerson**](PeopleAPI.md#updateperson) | **PATCH** /households/{household_id}/people/{id} | Update a person.
[**updatePersonWithPut**](PeopleAPI.md#updatepersonwithput) | **PUT** /households/{household_id}/people/{id} | Update a person using the PUT route.


# **createPerson**
```swift
    open class func createPerson(householdId: Int, personCreateRequest: PersonCreateRequest, completion: @escaping (_ data: PersonResponse?, _ error: Error?) -> Void)
```

Create a person in the household.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let personCreateRequest = PersonCreateRequest(person: PersonCreateRequest_person(name: "name_example", dateOfBirth: Date(), email: "email_example", personType: "personType_example", hasCapacity: false)) // PersonCreateRequest | 

// Create a person in the household.
PeopleAPI.createPerson(householdId: householdId, personCreateRequest: personCreateRequest) { (response, error) in
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
 **personCreateRequest** | [**PersonCreateRequest**](PersonCreateRequest.md) |  | 

### Return type

[**PersonResponse**](PersonResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getCurrentProfile**
```swift
    open class func getCurrentProfile(householdId: Int, completion: @escaping (_ data: MeResponse?, _ error: Error?) -> Void)
```

Read the current account and person profile for a household session.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read the current account and person profile for a household session.
PeopleAPI.getCurrentProfile(householdId: householdId) { (response, error) in
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

[**MeResponse**](MeResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getPerson**
```swift
    open class func getPerson(householdId: Int, id: Int, completion: @escaping (_ data: PersonResponse?, _ error: Error?) -> Void)
```

Read a person.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 

// Read a person.
PeopleAPI.getPerson(householdId: householdId, id: id) { (response, error) in
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

[**PersonResponse**](PersonResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listPeople**
```swift
    open class func listPeople(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: PersonCollectionResponse?, _ error: Error?) -> Void)
```

List visible people in the household.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible people in the household.
PeopleAPI.listPeople(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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
 **page** | **Int** |  | [optional] [default to 1]
 **perPage** | **Int** |  | [optional] [default to 20]
 **updatedSince** | **Date** |  | [optional] 

### Return type

[**PersonCollectionResponse**](PersonCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updatePerson**
```swift
    open class func updatePerson(householdId: Int, id: Int, personUpdateRequest: PersonUpdateRequest, completion: @escaping (_ data: PersonResponse?, _ error: Error?) -> Void)
```

Update a person.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 
let personUpdateRequest = PersonUpdateRequest(person: PersonAttributes(name: "name_example", email: "email_example", dateOfBirth: Date(), personType: "personType_example", hasCapacity: false)) // PersonUpdateRequest | 

// Update a person.
PeopleAPI.updatePerson(householdId: householdId, id: id, personUpdateRequest: personUpdateRequest) { (response, error) in
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
 **personUpdateRequest** | [**PersonUpdateRequest**](PersonUpdateRequest.md) |  | 

### Return type

[**PersonResponse**](PersonResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updatePersonWithPut**
```swift
    open class func updatePersonWithPut(householdId: Int, id: Int, personUpdateRequest: PersonUpdateRequest, completion: @escaping (_ data: PersonResponse?, _ error: Error?) -> Void)
```

Update a person using the PUT route.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = 987 // Int | 
let personUpdateRequest = PersonUpdateRequest(person: PersonAttributes(name: "name_example", email: "email_example", dateOfBirth: Date(), personType: "personType_example", hasCapacity: false)) // PersonUpdateRequest | 

// Update a person using the PUT route.
PeopleAPI.updatePersonWithPut(householdId: householdId, id: id, personUpdateRequest: personUpdateRequest) { (response, error) in
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
 **personUpdateRequest** | [**PersonUpdateRequest**](PersonUpdateRequest.md) |  | 

### Return type

[**PersonResponse**](PersonResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

