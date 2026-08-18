# NotificationPreferencesApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**getNotificationPreference**](NotificationPreferencesApi.md#getNotificationPreference) | **GET** /households/{household_id}/notification_preference | Read the signed-in person&#39;s notification preference. |
| [**replaceNotificationPreference**](NotificationPreferencesApi.md#replaceNotificationPreference) | **PUT** /households/{household_id}/notification_preference | Replace the signed-in person&#39;s notification preference. |
| [**updateNotificationPreference**](NotificationPreferencesApi.md#updateNotificationPreference) | **PATCH** /households/{household_id}/notification_preference | Update the signed-in person&#39;s notification preference. |


<a id="getNotificationPreference"></a>
# **getNotificationPreference**
> NotificationPreferenceResponse getNotificationPreference(householdId)

Read the signed-in person&#39;s notification preference.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = NotificationPreferencesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : NotificationPreferenceResponse = apiInstance.getNotificationPreference(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling NotificationPreferencesApi#getNotificationPreference")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling NotificationPreferencesApi#getNotificationPreference")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="replaceNotificationPreference"></a>
# **replaceNotificationPreference**
> NotificationPreferenceResponse replaceNotificationPreference(householdId, notificationPreferenceUpdateRequest)

Replace the signed-in person&#39;s notification preference.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = NotificationPreferencesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val notificationPreferenceUpdateRequest : NotificationPreferenceUpdateRequest =  // NotificationPreferenceUpdateRequest | 
try {
    val result : NotificationPreferenceResponse = apiInstance.replaceNotificationPreference(householdId, notificationPreferenceUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling NotificationPreferencesApi#replaceNotificationPreference")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling NotificationPreferencesApi#replaceNotificationPreference")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **notificationPreferenceUpdateRequest** | [**NotificationPreferenceUpdateRequest**](NotificationPreferenceUpdateRequest.md)|  | |

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updateNotificationPreference"></a>
# **updateNotificationPreference**
> NotificationPreferenceResponse updateNotificationPreference(householdId, notificationPreferenceUpdateRequest)

Update the signed-in person&#39;s notification preference.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = NotificationPreferencesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val notificationPreferenceUpdateRequest : NotificationPreferenceUpdateRequest =  // NotificationPreferenceUpdateRequest | 
try {
    val result : NotificationPreferenceResponse = apiInstance.updateNotificationPreference(householdId, notificationPreferenceUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling NotificationPreferencesApi#updateNotificationPreference")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling NotificationPreferencesApi#updateNotificationPreference")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **notificationPreferenceUpdateRequest** | [**NotificationPreferenceUpdateRequest**](NotificationPreferenceUpdateRequest.md)|  | |

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

