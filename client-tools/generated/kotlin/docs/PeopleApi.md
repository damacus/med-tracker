# PeopleApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createPerson**](PeopleApi.md#createPerson) | **POST** /households/{household_id}/people | Create a person in the household. |
| [**getCurrentProfile**](PeopleApi.md#getCurrentProfile) | **GET** /households/{household_id}/me | Read the current account and person profile for a household session. |
| [**getPerson**](PeopleApi.md#getPerson) | **GET** /households/{household_id}/people/{id} | Read a person. |
| [**listPeople**](PeopleApi.md#listPeople) | **GET** /households/{household_id}/people | List visible people in the household. |
| [**updatePerson**](PeopleApi.md#updatePerson) | **PATCH** /households/{household_id}/people/{id} | Update a person. |
| [**updatePersonWithPut**](PeopleApi.md#updatePersonWithPut) | **PUT** /households/{household_id}/people/{id} | Update a person using the PUT route. |


<a id="createPerson"></a>
# **createPerson**
> PersonResponse createPerson(householdId, personCreateRequest)

Create a person in the household.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PeopleApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val personCreateRequest : PersonCreateRequest =  // PersonCreateRequest | 
try {
    val result : PersonResponse = apiInstance.createPerson(householdId, personCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PeopleApi#createPerson")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PeopleApi#createPerson")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personCreateRequest** | [**PersonCreateRequest**](PersonCreateRequest.md)|  | |

### Return type

[**PersonResponse**](PersonResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="getCurrentProfile"></a>
# **getCurrentProfile**
> MeResponse getCurrentProfile(householdId)

Read the current account and person profile for a household session.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PeopleApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : MeResponse = apiInstance.getCurrentProfile(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PeopleApi#getCurrentProfile")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PeopleApi#getCurrentProfile")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**MeResponse**](MeResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getPerson"></a>
# **getPerson**
> PersonResponse getPerson(householdId, id)

Read a person.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PeopleApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : PersonResponse = apiInstance.getPerson(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PeopleApi#getPerson")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PeopleApi#getPerson")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int**|  | |

### Return type

[**PersonResponse**](PersonResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listPeople"></a>
# **listPeople**
> PersonCollectionResponse listPeople(householdId, page, perPage, updatedSince)

List visible people in the household.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PeopleApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : PersonCollectionResponse = apiInstance.listPeople(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PeopleApi#listPeople")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PeopleApi#listPeople")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **page** | **kotlin.Int**|  | [optional] [default to 1] |
| **perPage** | **kotlin.Int**|  | [optional] [default to 20] |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **updatedSince** | **java.time.OffsetDateTime**|  | [optional] |

### Return type

[**PersonCollectionResponse**](PersonCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="updatePerson"></a>
# **updatePerson**
> PersonResponse updatePerson(householdId, id, personUpdateRequest)

Update a person.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PeopleApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val personUpdateRequest : PersonUpdateRequest =  // PersonUpdateRequest | 
try {
    val result : PersonResponse = apiInstance.updatePerson(householdId, id, personUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PeopleApi#updatePerson")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PeopleApi#updatePerson")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personUpdateRequest** | [**PersonUpdateRequest**](PersonUpdateRequest.md)|  | |

### Return type

[**PersonResponse**](PersonResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updatePersonWithPut"></a>
# **updatePersonWithPut**
> PersonResponse updatePersonWithPut(householdId, id, personUpdateRequest)

Update a person using the PUT route.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = PeopleApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val personUpdateRequest : PersonUpdateRequest =  // PersonUpdateRequest | 
try {
    val result : PersonResponse = apiInstance.updatePersonWithPut(householdId, id, personUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling PeopleApi#updatePersonWithPut")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling PeopleApi#updatePersonWithPut")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personUpdateRequest** | [**PersonUpdateRequest**](PersonUpdateRequest.md)|  | |

### Return type

[**PersonResponse**](PersonResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

