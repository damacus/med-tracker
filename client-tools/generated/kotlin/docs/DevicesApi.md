# DevicesApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**createNativeDeviceToken**](DevicesApi.md#createNativeDeviceToken) | **POST** /households/{household_id}/native_device_tokens | Register a native device token. |
| [**createPushSubscription**](DevicesApi.md#createPushSubscription) | **POST** /households/{household_id}/push_subscription | Register a web push subscription. |
| [**deleteNativeDeviceToken**](DevicesApi.md#deleteNativeDeviceToken) | **DELETE** /households/{household_id}/native_device_tokens/{id} | Revoke a native device token. |
| [**deletePushSubscription**](DevicesApi.md#deletePushSubscription) | **DELETE** /households/{household_id}/push_subscription | Revoke a web push subscription. |
| [**testPushSubscription**](DevicesApi.md#testPushSubscription) | **POST** /households/{household_id}/push_subscription/test | Send a test push notification. |


<a id="createNativeDeviceToken"></a>
# **createNativeDeviceToken**
> createNativeDeviceToken(householdId, nativeDeviceTokenCreateRequest)

Register a native device token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DevicesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val nativeDeviceTokenCreateRequest : NativeDeviceTokenCreateRequest =  // NativeDeviceTokenCreateRequest | 
try {
    apiInstance.createNativeDeviceToken(householdId, nativeDeviceTokenCreateRequest)
} catch (e: ClientException) {
    println("4xx response calling DevicesApi#createNativeDeviceToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DevicesApi#createNativeDeviceToken")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **nativeDeviceTokenCreateRequest** | [**NativeDeviceTokenCreateRequest**](NativeDeviceTokenCreateRequest.md)|  | |

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createPushSubscription"></a>
# **createPushSubscription**
> createPushSubscription(householdId, pushSubscriptionCreateRequest)

Register a web push subscription.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DevicesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val pushSubscriptionCreateRequest : PushSubscriptionCreateRequest =  // PushSubscriptionCreateRequest | 
try {
    apiInstance.createPushSubscription(householdId, pushSubscriptionCreateRequest)
} catch (e: ClientException) {
    println("4xx response calling DevicesApi#createPushSubscription")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DevicesApi#createPushSubscription")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **pushSubscriptionCreateRequest** | [**PushSubscriptionCreateRequest**](PushSubscriptionCreateRequest.md)|  | |

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="deleteNativeDeviceToken"></a>
# **deleteNativeDeviceToken**
> deleteNativeDeviceToken(householdId, id)

Revoke a native device token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DevicesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    apiInstance.deleteNativeDeviceToken(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling DevicesApi#deleteNativeDeviceToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DevicesApi#deleteNativeDeviceToken")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="deletePushSubscription"></a>
# **deletePushSubscription**
> deletePushSubscription(householdId, endpoint)

Revoke a web push subscription.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DevicesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val endpoint : java.net.URI = endpoint_example // java.net.URI | 
try {
    apiInstance.deletePushSubscription(householdId, endpoint)
} catch (e: ClientException) {
    println("4xx response calling DevicesApi#deletePushSubscription")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DevicesApi#deletePushSubscription")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **endpoint** | **java.net.URI**|  | |

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="testPushSubscription"></a>
# **testPushSubscription**
> testPushSubscription(householdId)

Send a test push notification.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = DevicesApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.testPushSubscription(householdId)
} catch (e: ClientException) {
    println("4xx response calling DevicesApi#testPushSubscription")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling DevicesApi#testPushSubscription")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

null (empty response body)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

