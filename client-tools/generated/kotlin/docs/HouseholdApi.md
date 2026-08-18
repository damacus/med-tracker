# HouseholdApi

All URIs are relative to */api/v1*

| Method | HTTP request | Description |
| ------------- | ------------- | ------------- |
| [**adjustMedicationInventory**](HouseholdApi.md#adjustMedicationInventory) | **PATCH** /households/{household_id}/medications/{id}/adjust_inventory | Adjust medication inventory. |
| [**createDosageOption**](HouseholdApi.md#createDosageOption) | **POST** /households/{household_id}/dosage_options | Create a medication dosage option. |
| [**createHealthEvent**](HouseholdApi.md#createHealthEvent) | **POST** /households/{household_id}/health_events | Create a health event. |
| [**createMedication**](HouseholdApi.md#createMedication) | **POST** /households/{household_id}/medications | Create a medication. |
| [**createMedicationTake**](HouseholdApi.md#createMedicationTake) | **POST** /households/{household_id}/medication_takes | Record a medication take. |
| [**createNativeDeviceToken**](HouseholdApi.md#createNativeDeviceToken) | **POST** /households/{household_id}/native_device_tokens | Register a native device token. |
| [**createPerson**](HouseholdApi.md#createPerson) | **POST** /households/{household_id}/people | Create a person in the household. |
| [**createPersonMedication**](HouseholdApi.md#createPersonMedication) | **POST** /households/{household_id}/person_medications | Create a person medication assignment. |
| [**createPortableImport**](HouseholdApi.md#createPortableImport) | **POST** /households/{household_id}/portable_imports | Apply an encrypted portable import transactionally. |
| [**createPushSubscription**](HouseholdApi.md#createPushSubscription) | **POST** /households/{household_id}/push_subscription | Register a web push subscription. |
| [**createSchedule**](HouseholdApi.md#createSchedule) | **POST** /households/{household_id}/schedules | Create a schedule. |
| [**createSyncBatch**](HouseholdApi.md#createSyncBatch) | **POST** /households/{household_id}/sync/batches | Apply transactional sync batch operations. |
| [**deleteNativeDeviceToken**](HouseholdApi.md#deleteNativeDeviceToken) | **DELETE** /households/{household_id}/native_device_tokens/{id} | Revoke a native device token. |
| [**deletePushSubscription**](HouseholdApi.md#deletePushSubscription) | **DELETE** /households/{household_id}/push_subscription | Revoke a web push subscription. |
| [**dryRunPortableImport**](HouseholdApi.md#dryRunPortableImport) | **POST** /households/{household_id}/portable_imports/dry_run | Validate an encrypted portable import without writing data. |
| [**exportPortableHouseholdBundle**](HouseholdApi.md#exportPortableHouseholdBundle) | **GET** /households/{household_id}/portable_export | Export an encrypted portable household bundle. |
| [**generateAiMedicationSuggestions**](HouseholdApi.md#generateAiMedicationSuggestions) | **POST** /households/{household_id}/ai_medication_suggestions | Generate medication setup suggestions. |
| [**getCurrentProfile**](HouseholdApi.md#getCurrentProfile) | **GET** /households/{household_id}/me | Read the current account and person profile for a household session. |
| [**getDataExport**](HouseholdApi.md#getDataExport) | **GET** /households/{household_id}/data_exports/{mode} | Export household data in a selected backup/profile mode. |
| [**getDosageOption**](HouseholdApi.md#getDosageOption) | **GET** /households/{household_id}/dosage_options/{id} | Read a medication dosage option. |
| [**getHealthEvent**](HouseholdApi.md#getHealthEvent) | **GET** /households/{household_id}/health_events/{id} | Read a health event. |
| [**getLocation**](HouseholdApi.md#getLocation) | **GET** /households/{household_id}/locations/{id} | Read a location. |
| [**getMedication**](HouseholdApi.md#getMedication) | **GET** /households/{household_id}/medications/{id} | Read a medication. |
| [**getMobileSnapshot**](HouseholdApi.md#getMobileSnapshot) | **GET** /households/{household_id}/mobile_snapshot | Read a plaintext mobile sync snapshot over authenticated transport. |
| [**getNotificationPreference**](HouseholdApi.md#getNotificationPreference) | **GET** /households/{household_id}/notification_preference | Read the signed-in person&#39;s notification preference. |
| [**getPerson**](HouseholdApi.md#getPerson) | **GET** /households/{household_id}/people/{id} | Read a person. |
| [**getPersonMedication**](HouseholdApi.md#getPersonMedication) | **GET** /households/{household_id}/person_medications/{id} | Read a person medication assignment. |
| [**getSchedule**](HouseholdApi.md#getSchedule) | **GET** /households/{household_id}/schedules/{id} | Read a schedule. |
| [**getSyncChanges**](HouseholdApi.md#getSyncChanges) | **GET** /households/{household_id}/sync/changes | Read sync changes after a cursor. |
| [**getSyncSnapshot**](HouseholdApi.md#getSyncSnapshot) | **GET** /households/{household_id}/sync/snapshot | Read a portable v2 sync snapshot. |
| [**listDosageOptions**](HouseholdApi.md#listDosageOptions) | **GET** /households/{household_id}/dosage_options | List medication dosage options. |
| [**listHealthEvents**](HouseholdApi.md#listHealthEvents) | **GET** /households/{household_id}/health_events | List visible health events. |
| [**listLocations**](HouseholdApi.md#listLocations) | **GET** /households/{household_id}/locations | List visible locations. |
| [**listMedicationTakes**](HouseholdApi.md#listMedicationTakes) | **GET** /households/{household_id}/medication_takes | List visible medication takes. |
| [**listMedications**](HouseholdApi.md#listMedications) | **GET** /households/{household_id}/medications | List visible medications. |
| [**listPeople**](HouseholdApi.md#listPeople) | **GET** /households/{household_id}/people | List visible people in the household. |
| [**listPersonMedications**](HouseholdApi.md#listPersonMedications) | **GET** /households/{household_id}/person_medications | List visible person medication assignments. |
| [**listSchedules**](HouseholdApi.md#listSchedules) | **GET** /households/{household_id}/schedules | List visible schedules. |
| [**markMedicationAsOrdered**](HouseholdApi.md#markMedicationAsOrdered) | **PATCH** /households/{household_id}/medications/{id}/mark_as_ordered | Mark a medication reorder as ordered. |
| [**markMedicationAsReceived**](HouseholdApi.md#markMedicationAsReceived) | **PATCH** /households/{household_id}/medications/{id}/mark_as_received | Mark a medication reorder as received. |
| [**pausePersonMedication**](HouseholdApi.md#pausePersonMedication) | **PATCH** /households/{household_id}/person_medications/{id}/pause | Pause a person medication assignment. |
| [**pauseSchedule**](HouseholdApi.md#pauseSchedule) | **PATCH** /households/{household_id}/schedules/{id}/pause | Pause a schedule. |
| [**reorderPersonMedication**](HouseholdApi.md#reorderPersonMedication) | **PATCH** /households/{household_id}/person_medications/{id}/reorder | Reorder a person medication assignment. |
| [**replaceDosageOption**](HouseholdApi.md#replaceDosageOption) | **PUT** /households/{household_id}/dosage_options/{id} | Replace a medication dosage option. |
| [**replaceHealthEvent**](HouseholdApi.md#replaceHealthEvent) | **PUT** /households/{household_id}/health_events/{id} | Replace a health event. |
| [**replaceMedication**](HouseholdApi.md#replaceMedication) | **PUT** /households/{household_id}/medications/{id} | Replace a medication. |
| [**replaceNotificationPreference**](HouseholdApi.md#replaceNotificationPreference) | **PUT** /households/{household_id}/notification_preference | Replace the signed-in person&#39;s notification preference. |
| [**replacePersonMedication**](HouseholdApi.md#replacePersonMedication) | **PUT** /households/{household_id}/person_medications/{id} | Replace a person medication assignment. |
| [**replaceSchedule**](HouseholdApi.md#replaceSchedule) | **PUT** /households/{household_id}/schedules/{id} | Replace a schedule. |
| [**resumePersonMedication**](HouseholdApi.md#resumePersonMedication) | **PATCH** /households/{household_id}/person_medications/{id}/resume | Resume a person medication assignment. |
| [**resumeSchedule**](HouseholdApi.md#resumeSchedule) | **PATCH** /households/{household_id}/schedules/{id}/resume | Resume a schedule. |
| [**searchMedicationLookup**](HouseholdApi.md#searchMedicationLookup) | **GET** /households/{household_id}/medication_lookup | Search external medication lookup data. |
| [**testPushSubscription**](HouseholdApi.md#testPushSubscription) | **POST** /households/{household_id}/push_subscription/test | Send a test push notification. |
| [**updateDosageOption**](HouseholdApi.md#updateDosageOption) | **PATCH** /households/{household_id}/dosage_options/{id} | Update a medication dosage option. |
| [**updateHealthEvent**](HouseholdApi.md#updateHealthEvent) | **PATCH** /households/{household_id}/health_events/{id} | Update a health event. |
| [**updateMedication**](HouseholdApi.md#updateMedication) | **PATCH** /households/{household_id}/medications/{id} | Update a medication. |
| [**updateNotificationPreference**](HouseholdApi.md#updateNotificationPreference) | **PATCH** /households/{household_id}/notification_preference | Update the signed-in person&#39;s notification preference. |
| [**updatePerson**](HouseholdApi.md#updatePerson) | **PATCH** /households/{household_id}/people/{id} | Update a person. |
| [**updatePersonMedication**](HouseholdApi.md#updatePersonMedication) | **PATCH** /households/{household_id}/person_medications/{id} | Update a person medication assignment. |
| [**updatePersonWithPut**](HouseholdApi.md#updatePersonWithPut) | **PUT** /households/{household_id}/people/{id} | Update a person using the PUT route. |
| [**updateSchedule**](HouseholdApi.md#updateSchedule) | **PATCH** /households/{household_id}/schedules/{id} | Update a schedule. |


<a id="adjustMedicationInventory"></a>
# **adjustMedicationInventory**
> MedicationResponse adjustMedicationInventory(householdId, id, medicationInventoryAdjustmentRequest)

Adjust medication inventory.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationInventoryAdjustmentRequest : MedicationInventoryAdjustmentRequest =  // MedicationInventoryAdjustmentRequest | 
try {
    val result : MedicationResponse = apiInstance.adjustMedicationInventory(householdId, id, medicationInventoryAdjustmentRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#adjustMedicationInventory")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#adjustMedicationInventory")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationInventoryAdjustmentRequest** | [**MedicationInventoryAdjustmentRequest**](MedicationInventoryAdjustmentRequest.md)|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createDosageOption"></a>
# **createDosageOption**
> DosageOptionResponse createDosageOption(householdId, dosageOptionCreateRequest)

Create a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val dosageOptionCreateRequest : DosageOptionCreateRequest =  // DosageOptionCreateRequest | 
try {
    val result : DosageOptionResponse = apiInstance.createDosageOption(householdId, dosageOptionCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **dosageOptionCreateRequest** | [**DosageOptionCreateRequest**](DosageOptionCreateRequest.md)|  | |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createHealthEvent"></a>
# **createHealthEvent**
> HealthEventResponse createHealthEvent(householdId, healthEventCreateRequest)

Create a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val healthEventCreateRequest : HealthEventCreateRequest =  // HealthEventCreateRequest | 
try {
    val result : HealthEventResponse = apiInstance.createHealthEvent(householdId, healthEventCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **healthEventCreateRequest** | [**HealthEventCreateRequest**](HealthEventCreateRequest.md)|  | |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createMedication"></a>
# **createMedication**
> MedicationResponse createMedication(householdId, medicationCreateRequest)

Create a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val medicationCreateRequest : MedicationCreateRequest =  // MedicationCreateRequest | 
try {
    val result : MedicationResponse = apiInstance.createMedication(householdId, medicationCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationCreateRequest** | [**MedicationCreateRequest**](MedicationCreateRequest.md)|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createMedicationTake"></a>
# **createMedicationTake**
> MedicationTakeResponse createMedicationTake(householdId, medicationTakeCreateRequest)

Record a medication take.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val medicationTakeCreateRequest : MedicationTakeCreateRequest =  // MedicationTakeCreateRequest | 
try {
    val result : MedicationTakeResponse = apiInstance.createMedicationTake(householdId, medicationTakeCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createMedicationTake")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createMedicationTake")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationTakeCreateRequest** | [**MedicationTakeCreateRequest**](MedicationTakeCreateRequest.md)|  | |

### Return type

[**MedicationTakeResponse**](MedicationTakeResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createNativeDeviceToken"></a>
# **createNativeDeviceToken**
> createNativeDeviceToken(householdId, nativeDeviceTokenCreateRequest)

Register a native device token.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val nativeDeviceTokenCreateRequest : NativeDeviceTokenCreateRequest =  // NativeDeviceTokenCreateRequest | 
try {
    apiInstance.createNativeDeviceToken(householdId, nativeDeviceTokenCreateRequest)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createNativeDeviceToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createNativeDeviceToken")
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

<a id="createPerson"></a>
# **createPerson**
> PersonResponse createPerson(householdId, personCreateRequest)

Create a person in the household.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val personCreateRequest : PersonCreateRequest =  // PersonCreateRequest | 
try {
    val result : PersonResponse = apiInstance.createPerson(householdId, personCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createPerson")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createPerson")
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

<a id="createPersonMedication"></a>
# **createPersonMedication**
> PersonMedicationResponse createPersonMedication(householdId, personMedicationCreateRequest)

Create a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val personMedicationCreateRequest : PersonMedicationCreateRequest =  // PersonMedicationCreateRequest | 
try {
    val result : PersonMedicationResponse = apiInstance.createPersonMedication(householdId, personMedicationCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createPersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createPersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personMedicationCreateRequest** | [**PersonMedicationCreateRequest**](PersonMedicationCreateRequest.md)|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createPortableImport"></a>
# **createPortableImport**
> PortableImportResultResponse createPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)

Apply an encrypted portable import transactionally.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
val portableImportRequest : PortableImportRequest =  // PortableImportRequest | 
try {
    val result : PortableImportResultResponse = apiInstance.createPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createPortableImport")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createPortableImport")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md)|  | |

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val pushSubscriptionCreateRequest : PushSubscriptionCreateRequest =  // PushSubscriptionCreateRequest | 
try {
    apiInstance.createPushSubscription(householdId, pushSubscriptionCreateRequest)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createPushSubscription")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createPushSubscription")
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

<a id="createSchedule"></a>
# **createSchedule**
> ScheduleResponse createSchedule(householdId, scheduleCreateRequest)

Create a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val scheduleCreateRequest : ScheduleCreateRequest =  // ScheduleCreateRequest | 
try {
    val result : ScheduleResponse = apiInstance.createSchedule(householdId, scheduleCreateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **scheduleCreateRequest** | [**ScheduleCreateRequest**](ScheduleCreateRequest.md)|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="createSyncBatch"></a>
# **createSyncBatch**
> SyncBatchResponse createSyncBatch(householdId, syncBatchRequest)

Apply transactional sync batch operations.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val syncBatchRequest : SyncBatchRequest =  // SyncBatchRequest | 
try {
    val result : SyncBatchResponse = apiInstance.createSyncBatch(householdId, syncBatchRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#createSyncBatch")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#createSyncBatch")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **syncBatchRequest** | [**SyncBatchRequest**](SyncBatchRequest.md)|  | |

### Return type

[**SyncBatchResponse**](SyncBatchResponse.md)

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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    apiInstance.deleteNativeDeviceToken(householdId, id)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#deleteNativeDeviceToken")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#deleteNativeDeviceToken")
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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val endpoint : java.net.URI = endpoint_example // java.net.URI | 
try {
    apiInstance.deletePushSubscription(householdId, endpoint)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#deletePushSubscription")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#deletePushSubscription")
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

<a id="dryRunPortableImport"></a>
# **dryRunPortableImport**
> PortableImportResultResponse dryRunPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)

Validate an encrypted portable import without writing data.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
val portableImportRequest : PortableImportRequest =  // PortableImportRequest | 
try {
    val result : PortableImportResultResponse = apiInstance.dryRunPortableImport(householdId, xMedTrackerPortablePassphrase, portableImportRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#dryRunPortableImport")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#dryRunPortableImport")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md)|  | |

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="exportPortableHouseholdBundle"></a>
# **exportPortableHouseholdBundle**
> PortableEnvelopeResponse exportPortableHouseholdBundle(householdId, xMedTrackerPortablePassphrase)

Export an encrypted portable household bundle.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
try {
    val result : PortableEnvelopeResponse = apiInstance.exportPortableHouseholdBundle(householdId, xMedTrackerPortablePassphrase)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#exportPortableHouseholdBundle")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#exportPortableHouseholdBundle")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | |

### Return type

[**PortableEnvelopeResponse**](PortableEnvelopeResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="generateAiMedicationSuggestions"></a>
# **generateAiMedicationSuggestions**
> AiMedicationSuggestionResponse generateAiMedicationSuggestions(householdId, aiMedicationSuggestionRequest)

Generate medication setup suggestions.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val aiMedicationSuggestionRequest : AiMedicationSuggestionRequest =  // AiMedicationSuggestionRequest | 
try {
    val result : AiMedicationSuggestionResponse = apiInstance.generateAiMedicationSuggestions(householdId, aiMedicationSuggestionRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#generateAiMedicationSuggestions")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#generateAiMedicationSuggestions")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **aiMedicationSuggestionRequest** | [**AiMedicationSuggestionRequest**](AiMedicationSuggestionRequest.md)|  | [optional] |

### Return type

[**AiMedicationSuggestionResponse**](AiMedicationSuggestionResponse.md)

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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : MeResponse = apiInstance.getCurrentProfile(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getCurrentProfile")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getCurrentProfile")
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

<a id="getDataExport"></a>
# **getDataExport**
> DataExportResponse getDataExport(householdId, mode, xMedTrackerPortablePassphrase)

Export household data in a selected backup/profile mode.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val mode : kotlin.String = mode_example // kotlin.String | 
val xMedTrackerPortablePassphrase : kotlin.String = xMedTrackerPortablePassphrase_example // kotlin.String | Required only for the encrypted migration bundle mode.
try {
    val result : DataExportResponse = apiInstance.getDataExport(householdId, mode, xMedTrackerPortablePassphrase)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getDataExport")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getDataExport")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **mode** | **kotlin.String**|  | [enum: encrypted_migration_bundle, backup_zip, health_data_json] |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **xMedTrackerPortablePassphrase** | **kotlin.String**| Required only for the encrypted migration bundle mode. | [optional] |

### Return type

[**DataExportResponse**](DataExportResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getDosageOption"></a>
# **getDosageOption**
> DosageOptionResponse getDosageOption(householdId, id)

Read a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : DosageOptionResponse = apiInstance.getDosageOption(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getHealthEvent"></a>
# **getHealthEvent**
> HealthEventResponse getHealthEvent(householdId, id)

Read a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : HealthEventResponse = apiInstance.getHealthEvent(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getLocation"></a>
# **getLocation**
> LocationResponse getLocation(householdId, id)

Read a location.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : LocationResponse = apiInstance.getLocation(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getLocation")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getLocation")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**LocationResponse**](LocationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getMedication"></a>
# **getMedication**
> MedicationResponse getMedication(householdId, id)

Read a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.getMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getMobileSnapshot"></a>
# **getMobileSnapshot**
> PortableSnapshotResponse getMobileSnapshot(householdId)

Read a plaintext mobile sync snapshot over authenticated transport.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : PortableSnapshotResponse = apiInstance.getMobileSnapshot(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getMobileSnapshot")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getMobileSnapshot")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**PortableSnapshotResponse**](PortableSnapshotResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getNotificationPreference"></a>
# **getNotificationPreference**
> NotificationPreferenceResponse getNotificationPreference(householdId)

Read the signed-in person&#39;s notification preference.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : NotificationPreferenceResponse = apiInstance.getNotificationPreference(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getNotificationPreference")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getNotificationPreference")
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

<a id="getPerson"></a>
# **getPerson**
> PersonResponse getPerson(householdId, id)

Read a person.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : PersonResponse = apiInstance.getPerson(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getPerson")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getPerson")
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

<a id="getPersonMedication"></a>
# **getPersonMedication**
> PersonMedicationResponse getPersonMedication(householdId, id)

Read a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.getPersonMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getPersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getPersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getSchedule"></a>
# **getSchedule**
> ScheduleResponse getSchedule(householdId, id)

Read a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.getSchedule(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getSyncChanges"></a>
# **getSyncChanges**
> SyncChangesResponse getSyncChanges(householdId, cursor)

Read sync changes after a cursor.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val cursor : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | Return changes and tombstones at or after this timestamp.
try {
    val result : SyncChangesResponse = apiInstance.getSyncChanges(householdId, cursor)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getSyncChanges")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getSyncChanges")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **cursor** | **java.time.OffsetDateTime**| Return changes and tombstones at or after this timestamp. | |

### Return type

[**SyncChangesResponse**](SyncChangesResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="getSyncSnapshot"></a>
# **getSyncSnapshot**
> SyncSnapshotResponse getSyncSnapshot(householdId)

Read a portable v2 sync snapshot.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    val result : SyncSnapshotResponse = apiInstance.getSyncSnapshot(householdId)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#getSyncSnapshot")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#getSyncSnapshot")
    e.printStackTrace()
}
```

### Parameters
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **householdId** | **kotlin.Int**|  | |

### Return type

[**SyncSnapshotResponse**](SyncSnapshotResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listDosageOptions"></a>
# **listDosageOptions**
> DosageOptionCollectionResponse listDosageOptions(householdId, page, perPage, updatedSince)

List medication dosage options.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : DosageOptionCollectionResponse = apiInstance.listDosageOptions(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listDosageOptions")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listDosageOptions")
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

[**DosageOptionCollectionResponse**](DosageOptionCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listHealthEvents"></a>
# **listHealthEvents**
> HealthEventCollectionResponse listHealthEvents(householdId, page, perPage, updatedSince)

List visible health events.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : HealthEventCollectionResponse = apiInstance.listHealthEvents(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listHealthEvents")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listHealthEvents")
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

[**HealthEventCollectionResponse**](HealthEventCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listLocations"></a>
# **listLocations**
> LocationCollectionResponse listLocations(householdId, page, perPage, updatedSince)

List visible locations.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : LocationCollectionResponse = apiInstance.listLocations(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listLocations")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listLocations")
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

[**LocationCollectionResponse**](LocationCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listMedicationTakes"></a>
# **listMedicationTakes**
> MedicationTakeCollectionResponse listMedicationTakes(householdId, page, perPage, updatedSince)

List visible medication takes.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : MedicationTakeCollectionResponse = apiInstance.listMedicationTakes(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listMedicationTakes")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listMedicationTakes")
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

[**MedicationTakeCollectionResponse**](MedicationTakeCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listMedications"></a>
# **listMedications**
> MedicationCollectionResponse listMedications(householdId, page, perPage, updatedSince)

List visible medications.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : MedicationCollectionResponse = apiInstance.listMedications(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listMedications")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listMedications")
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

[**MedicationCollectionResponse**](MedicationCollectionResponse.md)

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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : PersonCollectionResponse = apiInstance.listPeople(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listPeople")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listPeople")
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

<a id="listPersonMedications"></a>
# **listPersonMedications**
> PersonMedicationCollectionResponse listPersonMedications(householdId, page, perPage, updatedSince)

List visible person medication assignments.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : PersonMedicationCollectionResponse = apiInstance.listPersonMedications(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listPersonMedications")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listPersonMedications")
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

[**PersonMedicationCollectionResponse**](PersonMedicationCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="listSchedules"></a>
# **listSchedules**
> ScheduleCollectionResponse listSchedules(householdId, page, perPage, updatedSince)

List visible schedules.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val page : kotlin.Int = 56 // kotlin.Int | 
val perPage : kotlin.Int = 56 // kotlin.Int | 
val updatedSince : java.time.OffsetDateTime = 2013-10-20T19:20:30+01:00 // java.time.OffsetDateTime | 
try {
    val result : ScheduleCollectionResponse = apiInstance.listSchedules(householdId, page, perPage, updatedSince)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#listSchedules")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#listSchedules")
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

[**ScheduleCollectionResponse**](ScheduleCollectionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="markMedicationAsOrdered"></a>
# **markMedicationAsOrdered**
> MedicationResponse markMedicationAsOrdered(householdId, id, medicationOrderDetailsRequest)

Mark a medication reorder as ordered.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationOrderDetailsRequest : MedicationOrderDetailsRequest =  // MedicationOrderDetailsRequest | 
try {
    val result : MedicationResponse = apiInstance.markMedicationAsOrdered(householdId, id, medicationOrderDetailsRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#markMedicationAsOrdered")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#markMedicationAsOrdered")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **medicationOrderDetailsRequest** | [**MedicationOrderDetailsRequest**](MedicationOrderDetailsRequest.md)|  | [optional] |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="markMedicationAsReceived"></a>
# **markMedicationAsReceived**
> MedicationResponse markMedicationAsReceived(householdId, id)

Mark a medication reorder as received.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.markMedicationAsReceived(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#markMedicationAsReceived")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#markMedicationAsReceived")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="pausePersonMedication"></a>
# **pausePersonMedication**
> PersonMedicationResponse pausePersonMedication(householdId, id)

Pause a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.pausePersonMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#pausePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#pausePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="pauseSchedule"></a>
# **pauseSchedule**
> ScheduleResponse pauseSchedule(householdId, id)

Pause a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.pauseSchedule(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#pauseSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#pauseSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="reorderPersonMedication"></a>
# **reorderPersonMedication**
> PersonMedicationResponse reorderPersonMedication(householdId, id, personMedicationReorderRequest)

Reorder a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val personMedicationReorderRequest : PersonMedicationReorderRequest =  // PersonMedicationReorderRequest | 
try {
    val result : PersonMedicationResponse = apiInstance.reorderPersonMedication(householdId, id, personMedicationReorderRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#reorderPersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#reorderPersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **personMedicationReorderRequest** | [**PersonMedicationReorderRequest**](PersonMedicationReorderRequest.md)|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="replaceDosageOption"></a>
# **replaceDosageOption**
> DosageOptionResponse replaceDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)

Replace a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val dosageOptionUpdateRequest : DosageOptionUpdateRequest =  // DosageOptionUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : DosageOptionResponse = apiInstance.replaceDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#replaceDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#replaceDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="replaceHealthEvent"></a>
# **replaceHealthEvent**
> HealthEventResponse replaceHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)

Replace a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val healthEventUpdateRequest : HealthEventUpdateRequest =  // HealthEventUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : HealthEventResponse = apiInstance.replaceHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#replaceHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#replaceHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="replaceMedication"></a>
# **replaceMedication**
> MedicationResponse replaceMedication(householdId, id, medicationUpdateRequest, ifMatch)

Replace a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationUpdateRequest : MedicationUpdateRequest =  // MedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.replaceMedication(householdId, id, medicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#replaceMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#replaceMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val notificationPreferenceUpdateRequest : NotificationPreferenceUpdateRequest =  // NotificationPreferenceUpdateRequest | 
try {
    val result : NotificationPreferenceResponse = apiInstance.replaceNotificationPreference(householdId, notificationPreferenceUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#replaceNotificationPreference")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#replaceNotificationPreference")
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

<a id="replacePersonMedication"></a>
# **replacePersonMedication**
> PersonMedicationResponse replacePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)

Replace a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val personMedicationUpdateRequest : PersonMedicationUpdateRequest =  // PersonMedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.replacePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#replacePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#replacePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="replaceSchedule"></a>
# **replaceSchedule**
> ScheduleResponse replaceSchedule(householdId, id, scheduleUpdateRequest, ifMatch)

Replace a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val scheduleUpdateRequest : ScheduleUpdateRequest =  // ScheduleUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.replaceSchedule(householdId, id, scheduleUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#replaceSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#replaceSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="resumePersonMedication"></a>
# **resumePersonMedication**
> PersonMedicationResponse resumePersonMedication(householdId, id)

Resume a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.resumePersonMedication(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#resumePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#resumePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="resumeSchedule"></a>
# **resumeSchedule**
> ScheduleResponse resumeSchedule(householdId, id)

Resume a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.resumeSchedule(householdId, id)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#resumeSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#resumeSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **id** | **kotlin.String**|  | |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

<a id="searchMedicationLookup"></a>
# **searchMedicationLookup**
> MedicationLookupResponse searchMedicationLookup(householdId, q, form, strength)

Search external medication lookup data.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val q : kotlin.String = q_example // kotlin.String | Medicine name, dm+d code, or barcode to search for.
val form : kotlin.String = form_example // kotlin.String | Dosage form used to filter search results.
val strength : kotlin.String = strength_example // kotlin.String | Medicine strength used to filter search results.
try {
    val result : MedicationLookupResponse = apiInstance.searchMedicationLookup(householdId, q, form, strength)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#searchMedicationLookup")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#searchMedicationLookup")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **q** | **kotlin.String**| Medicine name, dm+d code, or barcode to search for. | [optional] |
| **form** | **kotlin.String**| Dosage form used to filter search results. | [optional] |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **strength** | **kotlin.String**| Medicine strength used to filter search results. | [optional] |

### Return type

[**MedicationLookupResponse**](MedicationLookupResponse.md)

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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
try {
    apiInstance.testPushSubscription(householdId)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#testPushSubscription")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#testPushSubscription")
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

<a id="updateDosageOption"></a>
# **updateDosageOption**
> DosageOptionResponse updateDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)

Update a medication dosage option.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val dosageOptionUpdateRequest : DosageOptionUpdateRequest =  // DosageOptionUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : DosageOptionResponse = apiInstance.updateDosageOption(householdId, id, dosageOptionUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updateDosageOption")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updateDosageOption")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updateHealthEvent"></a>
# **updateHealthEvent**
> HealthEventResponse updateHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)

Update a health event.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val healthEventUpdateRequest : HealthEventUpdateRequest =  // HealthEventUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : HealthEventResponse = apiInstance.updateHealthEvent(householdId, id, healthEventUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updateHealthEvent")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updateHealthEvent")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

<a id="updateMedication"></a>
# **updateMedication**
> MedicationResponse updateMedication(householdId, id, medicationUpdateRequest, ifMatch)

Update a medication.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val medicationUpdateRequest : MedicationUpdateRequest =  // MedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : MedicationResponse = apiInstance.updateMedication(householdId, id, medicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updateMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updateMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**MedicationResponse**](MedicationResponse.md)

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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val notificationPreferenceUpdateRequest : NotificationPreferenceUpdateRequest =  // NotificationPreferenceUpdateRequest | 
try {
    val result : NotificationPreferenceResponse = apiInstance.updateNotificationPreference(householdId, notificationPreferenceUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updateNotificationPreference")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updateNotificationPreference")
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

<a id="updatePerson"></a>
# **updatePerson**
> PersonResponse updatePerson(householdId, id, personUpdateRequest)

Update a person.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val personUpdateRequest : PersonUpdateRequest =  // PersonUpdateRequest | 
try {
    val result : PersonResponse = apiInstance.updatePerson(householdId, id, personUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updatePerson")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updatePerson")
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

<a id="updatePersonMedication"></a>
# **updatePersonMedication**
> PersonMedicationResponse updatePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)

Update a person medication assignment.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val personMedicationUpdateRequest : PersonMedicationUpdateRequest =  // PersonMedicationUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : PersonMedicationResponse = apiInstance.updatePersonMedication(householdId, id, personMedicationUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updatePersonMedication")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updatePersonMedication")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

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

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.Int = 56 // kotlin.Int | 
val personUpdateRequest : PersonUpdateRequest =  // PersonUpdateRequest | 
try {
    val result : PersonResponse = apiInstance.updatePersonWithPut(householdId, id, personUpdateRequest)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updatePersonWithPut")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updatePersonWithPut")
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

<a id="updateSchedule"></a>
# **updateSchedule**
> ScheduleResponse updateSchedule(householdId, id, scheduleUpdateRequest, ifMatch)

Update a schedule.

### Example
```kotlin
// Import classes:
//import io.medtracker.client.infrastructure.*
//import io.medtracker.client.models.*

val apiInstance = HouseholdApi()
val householdId : kotlin.Int = 56 // kotlin.Int | 
val id : kotlin.String = id_example // kotlin.String | 
val scheduleUpdateRequest : ScheduleUpdateRequest =  // ScheduleUpdateRequest | 
val ifMatch : kotlin.String = ifMatch_example // kotlin.String | 
try {
    val result : ScheduleResponse = apiInstance.updateSchedule(householdId, id, scheduleUpdateRequest, ifMatch)
    println(result)
} catch (e: ClientException) {
    println("4xx response calling HouseholdApi#updateSchedule")
    e.printStackTrace()
} catch (e: ServerException) {
    println("5xx response calling HouseholdApi#updateSchedule")
    e.printStackTrace()
}
```

### Parameters
| **householdId** | **kotlin.Int**|  | |
| **id** | **kotlin.String**|  | |
| **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md)|  | |
| Name | Type | Description  | Notes |
| ------------- | ------------- | ------------- | ------------- |
| **ifMatch** | **kotlin.String**|  | [optional] |

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization


Configure bearerAuth:
    ApiClient.accessToken = ""

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

