# HouseholdSettingsApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**getHouseholdAdminSettings**](HouseholdSettingsApi.md#getHouseholdAdminSettings) | **GET** /households/{household_id}/admin/settings | Read household administration settings. |
| [**replaceHouseholdAdminSettings**](HouseholdSettingsApi.md#replaceHouseholdAdminSettings) | **PUT** /households/{household_id}/admin/settings | Replace household administration settings. |
| [**updateHouseholdAdminSettings**](HouseholdSettingsApi.md#updateHouseholdAdminSettings) | **PATCH** /households/{household_id}/admin/settings | Update household administration settings. |


<a id="getHouseholdAdminSettings"></a>
# **getHouseholdAdminSettings**
> HouseholdAdminSettingsResponse getHouseholdAdminSettings(householdId)

Read household administration settings.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdSettingsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : HouseholdAdminSettingsResponse = apiInstance.getHouseholdAdminSettings(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdSettingsApi#getHouseholdAdminSettings")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdSettingsApi#getHouseholdAdminSettings")
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

<a id="replaceHouseholdAdminSettings"></a>
# **replaceHouseholdAdminSettings**
> HouseholdAdminSettingsResponse replaceHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)

Replace household administration settings.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdSettingsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val householdAdminSettingsUpdateRequest : HouseholdAdminSettingsUpdateRequest =  // HouseholdAdminSettingsUpdateRequest | 
try {
    val result : HouseholdAdminSettingsResponse = apiInstance.replaceHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdSettingsApi#replaceHouseholdAdminSettings")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdSettingsApi#replaceHouseholdAdminSettings")
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

<a id="updateHouseholdAdminSettings"></a>
# **updateHouseholdAdminSettings**
> HouseholdAdminSettingsResponse updateHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)

Update household administration settings.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdSettingsApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val householdAdminSettingsUpdateRequest : HouseholdAdminSettingsUpdateRequest =  // HouseholdAdminSettingsUpdateRequest | 
try {
    val result : HouseholdAdminSettingsResponse = apiInstance.updateHouseholdAdminSettings(householdId, householdAdminSettingsUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdSettingsApi#updateHouseholdAdminSettings")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdSettingsApi#updateHouseholdAdminSettings")
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

