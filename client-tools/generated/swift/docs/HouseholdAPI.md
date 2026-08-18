# HouseholdAPI

All URIs are relative to */api/v1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**adjustMedicationInventory**](HouseholdAPI.md#adjustmedicationinventory) | **PATCH** /households/{household_id}/medications/{id}/adjust_inventory | Adjust medication inventory.
[**createDosageOption**](HouseholdAPI.md#createdosageoption) | **POST** /households/{household_id}/dosage_options | Create a medication dosage option.
[**createHealthEvent**](HouseholdAPI.md#createhealthevent) | **POST** /households/{household_id}/health_events | Create a health event.
[**createMedication**](HouseholdAPI.md#createmedication) | **POST** /households/{household_id}/medications | Create a medication.
[**createMedicationTake**](HouseholdAPI.md#createmedicationtake) | **POST** /households/{household_id}/medication_takes | Record a medication take.
[**createNativeDeviceToken**](HouseholdAPI.md#createnativedevicetoken) | **POST** /households/{household_id}/native_device_tokens | Register a native device token.
[**createPerson**](HouseholdAPI.md#createperson) | **POST** /households/{household_id}/people | Create a person in the household.
[**createPersonMedication**](HouseholdAPI.md#createpersonmedication) | **POST** /households/{household_id}/person_medications | Create a person medication assignment.
[**createPortableImport**](HouseholdAPI.md#createportableimport) | **POST** /households/{household_id}/portable_imports | Apply an encrypted portable import transactionally.
[**createPushSubscription**](HouseholdAPI.md#createpushsubscription) | **POST** /households/{household_id}/push_subscription | Register a web push subscription.
[**createSchedule**](HouseholdAPI.md#createschedule) | **POST** /households/{household_id}/schedules | Create a schedule.
[**createSyncBatch**](HouseholdAPI.md#createsyncbatch) | **POST** /households/{household_id}/sync/batches | Apply transactional sync batch operations.
[**deleteNativeDeviceToken**](HouseholdAPI.md#deletenativedevicetoken) | **DELETE** /households/{household_id}/native_device_tokens/{id} | Revoke a native device token.
[**deletePushSubscription**](HouseholdAPI.md#deletepushsubscription) | **DELETE** /households/{household_id}/push_subscription | Revoke a web push subscription.
[**dryRunPortableImport**](HouseholdAPI.md#dryrunportableimport) | **POST** /households/{household_id}/portable_imports/dry_run | Validate an encrypted portable import without writing data.
[**exportPortableHouseholdBundle**](HouseholdAPI.md#exportportablehouseholdbundle) | **GET** /households/{household_id}/portable_export | Export an encrypted portable household bundle.
[**generateAiMedicationSuggestions**](HouseholdAPI.md#generateaimedicationsuggestions) | **POST** /households/{household_id}/ai_medication_suggestions | Generate medication setup suggestions.
[**getCurrentProfile**](HouseholdAPI.md#getcurrentprofile) | **GET** /households/{household_id}/me | Read the current account and person profile for a household session.
[**getDataExport**](HouseholdAPI.md#getdataexport) | **GET** /households/{household_id}/data_exports/{mode} | Export household data in a selected backup/profile mode.
[**getDosageOption**](HouseholdAPI.md#getdosageoption) | **GET** /households/{household_id}/dosage_options/{id} | Read a medication dosage option.
[**getHealthEvent**](HouseholdAPI.md#gethealthevent) | **GET** /households/{household_id}/health_events/{id} | Read a health event.
[**getLocation**](HouseholdAPI.md#getlocation) | **GET** /households/{household_id}/locations/{id} | Read a location.
[**getMedication**](HouseholdAPI.md#getmedication) | **GET** /households/{household_id}/medications/{id} | Read a medication.
[**getMobileSnapshot**](HouseholdAPI.md#getmobilesnapshot) | **GET** /households/{household_id}/mobile_snapshot | Read a plaintext mobile sync snapshot over authenticated transport.
[**getNotificationPreference**](HouseholdAPI.md#getnotificationpreference) | **GET** /households/{household_id}/notification_preference | Read the signed-in person&#39;s notification preference.
[**getPerson**](HouseholdAPI.md#getperson) | **GET** /households/{household_id}/people/{id} | Read a person.
[**getPersonMedication**](HouseholdAPI.md#getpersonmedication) | **GET** /households/{household_id}/person_medications/{id} | Read a person medication assignment.
[**getSchedule**](HouseholdAPI.md#getschedule) | **GET** /households/{household_id}/schedules/{id} | Read a schedule.
[**getSyncChanges**](HouseholdAPI.md#getsyncchanges) | **GET** /households/{household_id}/sync/changes | Read sync changes after a cursor.
[**getSyncSnapshot**](HouseholdAPI.md#getsyncsnapshot) | **GET** /households/{household_id}/sync/snapshot | Read a portable v2 sync snapshot.
[**listDosageOptions**](HouseholdAPI.md#listdosageoptions) | **GET** /households/{household_id}/dosage_options | List medication dosage options.
[**listHealthEvents**](HouseholdAPI.md#listhealthevents) | **GET** /households/{household_id}/health_events | List visible health events.
[**listLocations**](HouseholdAPI.md#listlocations) | **GET** /households/{household_id}/locations | List visible locations.
[**listMedicationTakes**](HouseholdAPI.md#listmedicationtakes) | **GET** /households/{household_id}/medication_takes | List visible medication takes.
[**listMedications**](HouseholdAPI.md#listmedications) | **GET** /households/{household_id}/medications | List visible medications.
[**listPeople**](HouseholdAPI.md#listpeople) | **GET** /households/{household_id}/people | List visible people in the household.
[**listPersonMedications**](HouseholdAPI.md#listpersonmedications) | **GET** /households/{household_id}/person_medications | List visible person medication assignments.
[**listSchedules**](HouseholdAPI.md#listschedules) | **GET** /households/{household_id}/schedules | List visible schedules.
[**markMedicationAsOrdered**](HouseholdAPI.md#markmedicationasordered) | **PATCH** /households/{household_id}/medications/{id}/mark_as_ordered | Mark a medication reorder as ordered.
[**markMedicationAsReceived**](HouseholdAPI.md#markmedicationasreceived) | **PATCH** /households/{household_id}/medications/{id}/mark_as_received | Mark a medication reorder as received.
[**pausePersonMedication**](HouseholdAPI.md#pausepersonmedication) | **PATCH** /households/{household_id}/person_medications/{id}/pause | Pause a person medication assignment.
[**pauseSchedule**](HouseholdAPI.md#pauseschedule) | **PATCH** /households/{household_id}/schedules/{id}/pause | Pause a schedule.
[**reorderPersonMedication**](HouseholdAPI.md#reorderpersonmedication) | **PATCH** /households/{household_id}/person_medications/{id}/reorder | Reorder a person medication assignment.
[**replaceDosageOption**](HouseholdAPI.md#replacedosageoption) | **PUT** /households/{household_id}/dosage_options/{id} | Replace a medication dosage option.
[**replaceHealthEvent**](HouseholdAPI.md#replacehealthevent) | **PUT** /households/{household_id}/health_events/{id} | Replace a health event.
[**replaceMedication**](HouseholdAPI.md#replacemedication) | **PUT** /households/{household_id}/medications/{id} | Replace a medication.
[**replaceNotificationPreference**](HouseholdAPI.md#replacenotificationpreference) | **PUT** /households/{household_id}/notification_preference | Replace the signed-in person&#39;s notification preference.
[**replacePersonMedication**](HouseholdAPI.md#replacepersonmedication) | **PUT** /households/{household_id}/person_medications/{id} | Replace a person medication assignment.
[**replaceSchedule**](HouseholdAPI.md#replaceschedule) | **PUT** /households/{household_id}/schedules/{id} | Replace a schedule.
[**resumePersonMedication**](HouseholdAPI.md#resumepersonmedication) | **PATCH** /households/{household_id}/person_medications/{id}/resume | Resume a person medication assignment.
[**resumeSchedule**](HouseholdAPI.md#resumeschedule) | **PATCH** /households/{household_id}/schedules/{id}/resume | Resume a schedule.
[**searchMedicationLookup**](HouseholdAPI.md#searchmedicationlookup) | **GET** /households/{household_id}/medication_lookup | Search external medication lookup data.
[**testPushSubscription**](HouseholdAPI.md#testpushsubscription) | **POST** /households/{household_id}/push_subscription/test | Send a test push notification.
[**updateDosageOption**](HouseholdAPI.md#updatedosageoption) | **PATCH** /households/{household_id}/dosage_options/{id} | Update a medication dosage option.
[**updateHealthEvent**](HouseholdAPI.md#updatehealthevent) | **PATCH** /households/{household_id}/health_events/{id} | Update a health event.
[**updateMedication**](HouseholdAPI.md#updatemedication) | **PATCH** /households/{household_id}/medications/{id} | Update a medication.
[**updateNotificationPreference**](HouseholdAPI.md#updatenotificationpreference) | **PATCH** /households/{household_id}/notification_preference | Update the signed-in person&#39;s notification preference.
[**updatePerson**](HouseholdAPI.md#updateperson) | **PATCH** /households/{household_id}/people/{id} | Update a person.
[**updatePersonMedication**](HouseholdAPI.md#updatepersonmedication) | **PATCH** /households/{household_id}/person_medications/{id} | Update a person medication assignment.
[**updatePersonWithPut**](HouseholdAPI.md#updatepersonwithput) | **PUT** /households/{household_id}/people/{id} | Update a person using the PUT route.
[**updateSchedule**](HouseholdAPI.md#updateschedule) | **PATCH** /households/{household_id}/schedules/{id} | Update a schedule.


# **adjustMedicationInventory**
```swift
    open class func adjustMedicationInventory(householdId: Int, id: String, medicationInventoryAdjustmentRequest: MedicationInventoryAdjustmentRequest, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Adjust medication inventory.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationInventoryAdjustmentRequest = MedicationInventoryAdjustmentRequest(adjustment: MedicationInventoryAdjustmentRequest_adjustment(newQuantity: "newQuantity_example", reason: "reason_example")) // MedicationInventoryAdjustmentRequest | 

// Adjust medication inventory.
HouseholdAPI.adjustMedicationInventory(householdId: householdId, id: id, medicationInventoryAdjustmentRequest: medicationInventoryAdjustmentRequest) { (response, error) in
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
 **id** | **String** |  | 
 **medicationInventoryAdjustmentRequest** | [**MedicationInventoryAdjustmentRequest**](MedicationInventoryAdjustmentRequest.md) |  | 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createDosageOption**
```swift
    open class func createDosageOption(householdId: Int, dosageOptionCreateRequest: DosageOptionCreateRequest, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Create a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let dosageOptionCreateRequest = DosageOptionCreateRequest(dosageOption: DosageOptionCreateAttributes(medicationId: "medicationId_example", amount: "amount_example", unit: "unit_example", frequency: "frequency_example", defaultMaxDailyDoses: 123, defaultMinHoursBetweenDoses: "defaultMinHoursBetweenDoses_example", defaultDoseCycle: "defaultDoseCycle_example", description: "description_example", defaultForAdults: false, defaultForChildren: false, currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example")) // DosageOptionCreateRequest | 

// Create a medication dosage option.
HouseholdAPI.createDosageOption(householdId: householdId, dosageOptionCreateRequest: dosageOptionCreateRequest) { (response, error) in
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
 **dosageOptionCreateRequest** | [**DosageOptionCreateRequest**](DosageOptionCreateRequest.md) |  | 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createHealthEvent**
```swift
    open class func createHealthEvent(householdId: Int, healthEventCreateRequest: HealthEventCreateRequest, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Create a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let healthEventCreateRequest = HealthEventCreateRequest(healthEvent: HealthEventCreateRequest_health_event(personId: "personId_example", eventKind: "eventKind_example", title: "title_example", startedOn: Date(), severity: "severity_example", notes: "notes_example", endedOn: Date(), medicationIds: ["medicationIds_example"])) // HealthEventCreateRequest | 

// Create a health event.
HouseholdAPI.createHealthEvent(householdId: householdId, healthEventCreateRequest: healthEventCreateRequest) { (response, error) in
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
 **healthEventCreateRequest** | [**HealthEventCreateRequest**](HealthEventCreateRequest.md) |  | 

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createMedication**
```swift
    open class func createMedication(householdId: Int, medicationCreateRequest: MedicationCreateRequest, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Create a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let medicationCreateRequest = MedicationCreateRequest(medication: MedicationCreateAttributes(name: "name_example", reorderThreshold: "reorderThreshold_example", locationId: 123, friendlyName: "friendlyName_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", currentSupply: "currentSupply_example", warnings: "warnings_example", defaultScheduleType: "defaultScheduleType_example")) // MedicationCreateRequest | 

// Create a medication.
HouseholdAPI.createMedication(householdId: householdId, medicationCreateRequest: medicationCreateRequest) { (response, error) in
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
 **medicationCreateRequest** | [**MedicationCreateRequest**](MedicationCreateRequest.md) |  | 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createMedicationTake**
```swift
    open class func createMedicationTake(householdId: Int, medicationTakeCreateRequest: MedicationTakeCreateRequest, completion: @escaping (_ data: MedicationTakeResponse?, _ error: Error?) -> Void)
```

Record a medication take.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let medicationTakeCreateRequest = MedicationTakeCreateRequest(medicationTake: MedicationTakeCreateRequest_medication_take(sourceType: "sourceType_example", sourceId: "sourceId_example", takenAt: Date(), clientUuid: 123, doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", takenFromMedicationId: 123)) // MedicationTakeCreateRequest | 

// Record a medication take.
HouseholdAPI.createMedicationTake(householdId: householdId, medicationTakeCreateRequest: medicationTakeCreateRequest) { (response, error) in
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
 **medicationTakeCreateRequest** | [**MedicationTakeCreateRequest**](MedicationTakeCreateRequest.md) |  | 

### Return type

[**MedicationTakeResponse**](MedicationTakeResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createNativeDeviceToken**
```swift
    open class func createNativeDeviceToken(householdId: Int, nativeDeviceTokenCreateRequest: NativeDeviceTokenCreateRequest, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Register a native device token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let nativeDeviceTokenCreateRequest = NativeDeviceTokenCreateRequest(nativeDeviceToken: NativeDeviceTokenAttributes(deviceToken: "deviceToken_example", platform: "platform_example")) // NativeDeviceTokenCreateRequest | 

// Register a native device token.
HouseholdAPI.createNativeDeviceToken(householdId: householdId, nativeDeviceTokenCreateRequest: nativeDeviceTokenCreateRequest) { (response, error) in
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
 **nativeDeviceTokenCreateRequest** | [**NativeDeviceTokenCreateRequest**](NativeDeviceTokenCreateRequest.md) |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

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
HouseholdAPI.createPerson(householdId: householdId, personCreateRequest: personCreateRequest) { (response, error) in
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

# **createPersonMedication**
```swift
    open class func createPersonMedication(householdId: Int, personMedicationCreateRequest: PersonMedicationCreateRequest, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Create a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let personMedicationCreateRequest = PersonMedicationCreateRequest(personMedication: PersonMedicationCreateRequest_person_medication(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", administrationKind: "administrationKind_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example")) // PersonMedicationCreateRequest | 

// Create a person medication assignment.
HouseholdAPI.createPersonMedication(householdId: householdId, personMedicationCreateRequest: personMedicationCreateRequest) { (response, error) in
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
 **personMedicationCreateRequest** | [**PersonMedicationCreateRequest**](PersonMedicationCreateRequest.md) |  | 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createPortableImport**
```swift
    open class func createPortableImport(householdId: Int, xMedTrackerPortablePassphrase: String, portableImportRequest: PortableImportRequest, completion: @escaping (_ data: PortableImportResultResponse?, _ error: Error?) -> Void)
```

Apply an encrypted portable import transactionally.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
let portableImportRequest = PortableImportRequest(bundle: PortableEnvelope(format: "format_example", encryptedAt: Date(), cipher: "cipher_example", kdf: "kdf_example", salt: "salt_example", checksum: "checksum_example", ciphertext: "ciphertext_example")) // PortableImportRequest | 

// Apply an encrypted portable import transactionally.
HouseholdAPI.createPortableImport(householdId: householdId, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase, portableImportRequest: portableImportRequest) { (response, error) in
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
 **xMedTrackerPortablePassphrase** | **String** | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | 
 **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md) |  | 

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createPushSubscription**
```swift
    open class func createPushSubscription(householdId: Int, pushSubscriptionCreateRequest: PushSubscriptionCreateRequest, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Register a web push subscription.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let pushSubscriptionCreateRequest = PushSubscriptionCreateRequest(pushSubscription: PushSubscriptionAttributes(endpoint: "endpoint_example", keys: PushSubscriptionKeys(p256dh: "p256dh_example", auth: "auth_example"))) // PushSubscriptionCreateRequest | 

// Register a web push subscription.
HouseholdAPI.createPushSubscription(householdId: householdId, pushSubscriptionCreateRequest: pushSubscriptionCreateRequest) { (response, error) in
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
 **pushSubscriptionCreateRequest** | [**PushSubscriptionCreateRequest**](PushSubscriptionCreateRequest.md) |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createSchedule**
```swift
    open class func createSchedule(householdId: Int, scheduleCreateRequest: ScheduleCreateRequest, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Create a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let scheduleCreateRequest = ScheduleCreateRequest(schedule: ScheduleCreateRequest_schedule(personId: "personId_example", medicationId: "medicationId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", startDate: Date(), endDate: Date(), sourceDosageOptionId: "sourceDosageOptionId_example", frequency: "frequency_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example", scheduleType: "scheduleType_example", scheduleConfig: ScheduleConfig(times: ["times_example"], weekdays: ["weekdays_example"], dates: [Date()], asNeeded: false, taperSteps: [ScheduleTaperStep(startDate: Date(), endDate: Date(), amount: "amount_example", doseAmount: "doseAmount_example", unit: "unit_example", doseUnit: "doseUnit_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", times: ["times_example"])]))) // ScheduleCreateRequest | 

// Create a schedule.
HouseholdAPI.createSchedule(householdId: householdId, scheduleCreateRequest: scheduleCreateRequest) { (response, error) in
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
 **scheduleCreateRequest** | [**ScheduleCreateRequest**](ScheduleCreateRequest.md) |  | 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **createSyncBatch**
```swift
    open class func createSyncBatch(householdId: Int, syncBatchRequest: SyncBatchRequest, completion: @escaping (_ data: SyncBatchResponse?, _ error: Error?) -> Void)
```

Apply transactional sync batch operations.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let syncBatchRequest = SyncBatchRequest(batch: SyncBatchRequest_batch(operations: [SyncBatchOperation(action: "action_example", resourceType: "resourceType_example", id: "id_example", ifMatch: "ifMatch_example", attributes: "TODO")])) // SyncBatchRequest | 

// Apply transactional sync batch operations.
HouseholdAPI.createSyncBatch(householdId: householdId, syncBatchRequest: syncBatchRequest) { (response, error) in
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
 **syncBatchRequest** | [**SyncBatchRequest**](SyncBatchRequest.md) |  | 

### Return type

[**SyncBatchResponse**](SyncBatchResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteNativeDeviceToken**
```swift
    open class func deleteNativeDeviceToken(householdId: Int, id: String, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke a native device token.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Revoke a native device token.
HouseholdAPI.deleteNativeDeviceToken(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deletePushSubscription**
```swift
    open class func deletePushSubscription(householdId: Int, endpoint: String, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Revoke a web push subscription.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let endpoint = "endpoint_example" // String | 

// Revoke a web push subscription.
HouseholdAPI.deletePushSubscription(householdId: householdId, endpoint: endpoint) { (response, error) in
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
 **endpoint** | **String** |  | 

### Return type

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **dryRunPortableImport**
```swift
    open class func dryRunPortableImport(householdId: Int, xMedTrackerPortablePassphrase: String, portableImportRequest: PortableImportRequest, completion: @escaping (_ data: PortableImportResultResponse?, _ error: Error?) -> Void)
```

Validate an encrypted portable import without writing data.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.
let portableImportRequest = PortableImportRequest(bundle: PortableEnvelope(format: "format_example", encryptedAt: Date(), cipher: "cipher_example", kdf: "kdf_example", salt: "salt_example", checksum: "checksum_example", ciphertext: "ciphertext_example")) // PortableImportRequest | 

// Validate an encrypted portable import without writing data.
HouseholdAPI.dryRunPortableImport(householdId: householdId, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase, portableImportRequest: portableImportRequest) { (response, error) in
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
 **xMedTrackerPortablePassphrase** | **String** | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | 
 **portableImportRequest** | [**PortableImportRequest**](PortableImportRequest.md) |  | 

### Return type

[**PortableImportResultResponse**](PortableImportResultResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **exportPortableHouseholdBundle**
```swift
    open class func exportPortableHouseholdBundle(householdId: Int, xMedTrackerPortablePassphrase: String, completion: @escaping (_ data: PortableEnvelopeResponse?, _ error: Error?) -> Void)
```

Export an encrypted portable household bundle.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL.

// Export an encrypted portable household bundle.
HouseholdAPI.exportPortableHouseholdBundle(householdId: householdId, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase) { (response, error) in
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
 **xMedTrackerPortablePassphrase** | **String** | Passphrase used to encrypt or decrypt the portable bundle. It is never accepted in the URL. | 

### Return type

[**PortableEnvelopeResponse**](PortableEnvelopeResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **generateAiMedicationSuggestions**
```swift
    open class func generateAiMedicationSuggestions(householdId: Int, aiMedicationSuggestionRequest: AiMedicationSuggestionRequest? = nil, completion: @escaping (_ data: AiMedicationSuggestionResponse?, _ error: Error?) -> Void)
```

Generate medication setup suggestions.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let aiMedicationSuggestionRequest = AiMedicationSuggestionRequest(medication: AiMedicationIdentity(name: "name_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example")) // AiMedicationSuggestionRequest |  (optional)

// Generate medication setup suggestions.
HouseholdAPI.generateAiMedicationSuggestions(householdId: householdId, aiMedicationSuggestionRequest: aiMedicationSuggestionRequest) { (response, error) in
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
 **aiMedicationSuggestionRequest** | [**AiMedicationSuggestionRequest**](AiMedicationSuggestionRequest.md) |  | [optional] 

### Return type

[**AiMedicationSuggestionResponse**](AiMedicationSuggestionResponse.md)

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
HouseholdAPI.getCurrentProfile(householdId: householdId) { (response, error) in
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

# **getDataExport**
```swift
    open class func getDataExport(householdId: Int, mode: Mode_getDataExport, xMedTrackerPortablePassphrase: String? = nil, completion: @escaping (_ data: DataExportResponse?, _ error: Error?) -> Void)
```

Export household data in a selected backup/profile mode.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let mode = "mode_example" // String | 
let xMedTrackerPortablePassphrase = "xMedTrackerPortablePassphrase_example" // String | Required only for the encrypted migration bundle mode. (optional)

// Export household data in a selected backup/profile mode.
HouseholdAPI.getDataExport(householdId: householdId, mode: mode, xMedTrackerPortablePassphrase: xMedTrackerPortablePassphrase) { (response, error) in
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
 **mode** | **String** |  | 
 **xMedTrackerPortablePassphrase** | **String** | Required only for the encrypted migration bundle mode. | [optional] 

### Return type

[**DataExportResponse**](DataExportResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getDosageOption**
```swift
    open class func getDosageOption(householdId: Int, id: String, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Read a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a medication dosage option.
HouseholdAPI.getDosageOption(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getHealthEvent**
```swift
    open class func getHealthEvent(householdId: Int, id: String, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Read a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a health event.
HouseholdAPI.getHealthEvent(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getLocation**
```swift
    open class func getLocation(householdId: Int, id: String, completion: @escaping (_ data: LocationResponse?, _ error: Error?) -> Void)
```

Read a location.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a location.
HouseholdAPI.getLocation(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**LocationResponse**](LocationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getMedication**
```swift
    open class func getMedication(householdId: Int, id: String, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Read a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a medication.
HouseholdAPI.getMedication(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getMobileSnapshot**
```swift
    open class func getMobileSnapshot(householdId: Int, completion: @escaping (_ data: PortableSnapshotResponse?, _ error: Error?) -> Void)
```

Read a plaintext mobile sync snapshot over authenticated transport.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read a plaintext mobile sync snapshot over authenticated transport.
HouseholdAPI.getMobileSnapshot(householdId: householdId) { (response, error) in
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

[**PortableSnapshotResponse**](PortableSnapshotResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getNotificationPreference**
```swift
    open class func getNotificationPreference(householdId: Int, completion: @escaping (_ data: NotificationPreferenceResponse?, _ error: Error?) -> Void)
```

Read the signed-in person's notification preference.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read the signed-in person's notification preference.
HouseholdAPI.getNotificationPreference(householdId: householdId) { (response, error) in
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

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

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
HouseholdAPI.getPerson(householdId: householdId, id: id) { (response, error) in
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

# **getPersonMedication**
```swift
    open class func getPersonMedication(householdId: Int, id: String, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Read a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a person medication assignment.
HouseholdAPI.getPersonMedication(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSchedule**
```swift
    open class func getSchedule(householdId: Int, id: String, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Read a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Read a schedule.
HouseholdAPI.getSchedule(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSyncChanges**
```swift
    open class func getSyncChanges(householdId: Int, cursor: Date, completion: @escaping (_ data: SyncChangesResponse?, _ error: Error?) -> Void)
```

Read sync changes after a cursor.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let cursor = Date() // Date | Return changes and tombstones at or after this timestamp.

// Read sync changes after a cursor.
HouseholdAPI.getSyncChanges(householdId: householdId, cursor: cursor) { (response, error) in
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
 **cursor** | **Date** | Return changes and tombstones at or after this timestamp. | 

### Return type

[**SyncChangesResponse**](SyncChangesResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getSyncSnapshot**
```swift
    open class func getSyncSnapshot(householdId: Int, completion: @escaping (_ data: SyncSnapshotResponse?, _ error: Error?) -> Void)
```

Read a portable v2 sync snapshot.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Read a portable v2 sync snapshot.
HouseholdAPI.getSyncSnapshot(householdId: householdId) { (response, error) in
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

[**SyncSnapshotResponse**](SyncSnapshotResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listDosageOptions**
```swift
    open class func listDosageOptions(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: DosageOptionCollectionResponse?, _ error: Error?) -> Void)
```

List medication dosage options.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List medication dosage options.
HouseholdAPI.listDosageOptions(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**DosageOptionCollectionResponse**](DosageOptionCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listHealthEvents**
```swift
    open class func listHealthEvents(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: HealthEventCollectionResponse?, _ error: Error?) -> Void)
```

List visible health events.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible health events.
HouseholdAPI.listHealthEvents(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**HealthEventCollectionResponse**](HealthEventCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listLocations**
```swift
    open class func listLocations(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: LocationCollectionResponse?, _ error: Error?) -> Void)
```

List visible locations.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible locations.
HouseholdAPI.listLocations(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**LocationCollectionResponse**](LocationCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listMedicationTakes**
```swift
    open class func listMedicationTakes(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: MedicationTakeCollectionResponse?, _ error: Error?) -> Void)
```

List visible medication takes.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible medication takes.
HouseholdAPI.listMedicationTakes(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**MedicationTakeCollectionResponse**](MedicationTakeCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listMedications**
```swift
    open class func listMedications(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: MedicationCollectionResponse?, _ error: Error?) -> Void)
```

List visible medications.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible medications.
HouseholdAPI.listMedications(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**MedicationCollectionResponse**](MedicationCollectionResponse.md)

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
HouseholdAPI.listPeople(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

# **listPersonMedications**
```swift
    open class func listPersonMedications(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: PersonMedicationCollectionResponse?, _ error: Error?) -> Void)
```

List visible person medication assignments.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible person medication assignments.
HouseholdAPI.listPersonMedications(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**PersonMedicationCollectionResponse**](PersonMedicationCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listSchedules**
```swift
    open class func listSchedules(householdId: Int, page: Int? = nil, perPage: Int? = nil, updatedSince: Date? = nil, completion: @escaping (_ data: ScheduleCollectionResponse?, _ error: Error?) -> Void)
```

List visible schedules.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let page = 987 // Int |  (optional) (default to 1)
let perPage = 987 // Int |  (optional) (default to 20)
let updatedSince = Date() // Date |  (optional)

// List visible schedules.
HouseholdAPI.listSchedules(householdId: householdId, page: page, perPage: perPage, updatedSince: updatedSince) { (response, error) in
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

[**ScheduleCollectionResponse**](ScheduleCollectionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **markMedicationAsOrdered**
```swift
    open class func markMedicationAsOrdered(householdId: Int, id: String, medicationOrderDetailsRequest: MedicationOrderDetailsRequest? = nil, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Mark a medication reorder as ordered.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationOrderDetailsRequest = MedicationOrderDetailsRequest(orderDetails: MedicationOrderDetailsRequest_order_details(supplier: "supplier_example", quantity: "quantity_example", expectedArrivalOn: Date())) // MedicationOrderDetailsRequest |  (optional)

// Mark a medication reorder as ordered.
HouseholdAPI.markMedicationAsOrdered(householdId: householdId, id: id, medicationOrderDetailsRequest: medicationOrderDetailsRequest) { (response, error) in
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
 **id** | **String** |  | 
 **medicationOrderDetailsRequest** | [**MedicationOrderDetailsRequest**](MedicationOrderDetailsRequest.md) |  | [optional] 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **markMedicationAsReceived**
```swift
    open class func markMedicationAsReceived(householdId: Int, id: String, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Mark a medication reorder as received.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Mark a medication reorder as received.
HouseholdAPI.markMedicationAsReceived(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **pausePersonMedication**
```swift
    open class func pausePersonMedication(householdId: Int, id: String, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Pause a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Pause a person medication assignment.
HouseholdAPI.pausePersonMedication(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **pauseSchedule**
```swift
    open class func pauseSchedule(householdId: Int, id: String, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Pause a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Pause a schedule.
HouseholdAPI.pauseSchedule(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **reorderPersonMedication**
```swift
    open class func reorderPersonMedication(householdId: Int, id: String, personMedicationReorderRequest: PersonMedicationReorderRequest, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Reorder a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let personMedicationReorderRequest = PersonMedicationReorderRequest(direction: "direction_example") // PersonMedicationReorderRequest | 

// Reorder a person medication assignment.
HouseholdAPI.reorderPersonMedication(householdId: householdId, id: id, personMedicationReorderRequest: personMedicationReorderRequest) { (response, error) in
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
 **id** | **String** |  | 
 **personMedicationReorderRequest** | [**PersonMedicationReorderRequest**](PersonMedicationReorderRequest.md) |  | 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceDosageOption**
```swift
    open class func replaceDosageOption(householdId: Int, id: String, dosageOptionUpdateRequest: DosageOptionUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Replace a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let dosageOptionUpdateRequest = DosageOptionUpdateRequest(dosageOption: DosageOptionUpdateAttributes(amount: "amount_example", unit: "unit_example", frequency: "frequency_example", description: "description_example", defaultForAdults: false, defaultForChildren: false, defaultMaxDailyDoses: 123, defaultMinHoursBetweenDoses: "defaultMinHoursBetweenDoses_example", defaultDoseCycle: "defaultDoseCycle_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example")) // DosageOptionUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a medication dosage option.
HouseholdAPI.replaceDosageOption(householdId: householdId, id: id, dosageOptionUpdateRequest: dosageOptionUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceHealthEvent**
```swift
    open class func replaceHealthEvent(householdId: Int, id: String, healthEventUpdateRequest: HealthEventUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Replace a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let healthEventUpdateRequest = HealthEventUpdateRequest(healthEvent: HealthEventAttributes(personId: "personId_example", eventKind: "eventKind_example", severity: "severity_example", title: "title_example", notes: "notes_example", startedOn: Date(), endedOn: Date(), medicationIds: ["medicationIds_example"])) // HealthEventUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a health event.
HouseholdAPI.replaceHealthEvent(householdId: householdId, id: id, healthEventUpdateRequest: healthEventUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceMedication**
```swift
    open class func replaceMedication(householdId: Int, id: String, medicationUpdateRequest: MedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Replace a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationUpdateRequest = MedicationUpdateRequest(medication: MedicationUpdateAttributes(name: "name_example", friendlyName: "friendlyName_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example", warnings: "warnings_example", locationId: 123, defaultScheduleType: "defaultScheduleType_example")) // MedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a medication.
HouseholdAPI.replaceMedication(householdId: householdId, id: id, medicationUpdateRequest: medicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceNotificationPreference**
```swift
    open class func replaceNotificationPreference(householdId: Int, notificationPreferenceUpdateRequest: NotificationPreferenceUpdateRequest, completion: @escaping (_ data: NotificationPreferenceResponse?, _ error: Error?) -> Void)
```

Replace the signed-in person's notification preference.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let notificationPreferenceUpdateRequest = NotificationPreferenceUpdateRequest(notificationPreference: NotificationPreferenceAttributes(enabled: false, doseDueEnabled: false, missedDoseEnabled: false, lowStockEnabled: false, privateTextEnabled: false, morningTime: "morningTime_example", afternoonTime: "afternoonTime_example", eveningTime: "eveningTime_example", nightTime: "nightTime_example")) // NotificationPreferenceUpdateRequest | 

// Replace the signed-in person's notification preference.
HouseholdAPI.replaceNotificationPreference(householdId: householdId, notificationPreferenceUpdateRequest: notificationPreferenceUpdateRequest) { (response, error) in
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
 **notificationPreferenceUpdateRequest** | [**NotificationPreferenceUpdateRequest**](NotificationPreferenceUpdateRequest.md) |  | 

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replacePersonMedication**
```swift
    open class func replacePersonMedication(householdId: Int, id: String, personMedicationUpdateRequest: PersonMedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Replace a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let personMedicationUpdateRequest = PersonMedicationUpdateRequest(personMedication: PersonMedicationAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", administrationKind: "administrationKind_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example")) // PersonMedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a person medication assignment.
HouseholdAPI.replacePersonMedication(householdId: householdId, id: id, personMedicationUpdateRequest: personMedicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **replaceSchedule**
```swift
    open class func replaceSchedule(householdId: Int, id: String, scheduleUpdateRequest: ScheduleUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Replace a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let scheduleUpdateRequest = ScheduleUpdateRequest(schedule: ScheduleAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", frequency: "frequency_example", startDate: Date(), endDate: Date(), notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example", scheduleType: "scheduleType_example", scheduleConfig: ScheduleConfig(times: ["times_example"], weekdays: ["weekdays_example"], dates: [Date()], asNeeded: false, taperSteps: [ScheduleTaperStep(startDate: Date(), endDate: Date(), amount: "amount_example", doseAmount: "doseAmount_example", unit: "unit_example", doseUnit: "doseUnit_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", times: ["times_example"])]))) // ScheduleUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Replace a schedule.
HouseholdAPI.replaceSchedule(householdId: householdId, id: id, scheduleUpdateRequest: scheduleUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **resumePersonMedication**
```swift
    open class func resumePersonMedication(householdId: Int, id: String, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Resume a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Resume a person medication assignment.
HouseholdAPI.resumePersonMedication(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **resumeSchedule**
```swift
    open class func resumeSchedule(householdId: Int, id: String, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Resume a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 

// Resume a schedule.
HouseholdAPI.resumeSchedule(householdId: householdId, id: id) { (response, error) in
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
 **id** | **String** |  | 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **searchMedicationLookup**
```swift
    open class func searchMedicationLookup(householdId: Int, q: String? = nil, form: String? = nil, strength: String? = nil, completion: @escaping (_ data: MedicationLookupResponse?, _ error: Error?) -> Void)
```

Search external medication lookup data.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let q = "q_example" // String | Medicine name, dm+d code, or barcode to search for. (optional)
let form = "form_example" // String | Dosage form used to filter search results. (optional)
let strength = "strength_example" // String | Medicine strength used to filter search results. (optional)

// Search external medication lookup data.
HouseholdAPI.searchMedicationLookup(householdId: householdId, q: q, form: form, strength: strength) { (response, error) in
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
 **q** | **String** | Medicine name, dm+d code, or barcode to search for. | [optional] 
 **form** | **String** | Dosage form used to filter search results. | [optional] 
 **strength** | **String** | Medicine strength used to filter search results. | [optional] 

### Return type

[**MedicationLookupResponse**](MedicationLookupResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **testPushSubscription**
```swift
    open class func testPushSubscription(householdId: Int, completion: @escaping (_ data: Void?, _ error: Error?) -> Void)
```

Send a test push notification.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 

// Send a test push notification.
HouseholdAPI.testPushSubscription(householdId: householdId) { (response, error) in
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

Void (empty response body)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateDosageOption**
```swift
    open class func updateDosageOption(householdId: Int, id: String, dosageOptionUpdateRequest: DosageOptionUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: DosageOptionResponse?, _ error: Error?) -> Void)
```

Update a medication dosage option.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let dosageOptionUpdateRequest = DosageOptionUpdateRequest(dosageOption: DosageOptionUpdateAttributes(amount: "amount_example", unit: "unit_example", frequency: "frequency_example", description: "description_example", defaultForAdults: false, defaultForChildren: false, defaultMaxDailyDoses: 123, defaultMinHoursBetweenDoses: "defaultMinHoursBetweenDoses_example", defaultDoseCycle: "defaultDoseCycle_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example")) // DosageOptionUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a medication dosage option.
HouseholdAPI.updateDosageOption(householdId: householdId, id: id, dosageOptionUpdateRequest: dosageOptionUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **dosageOptionUpdateRequest** | [**DosageOptionUpdateRequest**](DosageOptionUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**DosageOptionResponse**](DosageOptionResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateHealthEvent**
```swift
    open class func updateHealthEvent(householdId: Int, id: String, healthEventUpdateRequest: HealthEventUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: HealthEventResponse?, _ error: Error?) -> Void)
```

Update a health event.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let healthEventUpdateRequest = HealthEventUpdateRequest(healthEvent: HealthEventAttributes(personId: "personId_example", eventKind: "eventKind_example", severity: "severity_example", title: "title_example", notes: "notes_example", startedOn: Date(), endedOn: Date(), medicationIds: ["medicationIds_example"])) // HealthEventUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a health event.
HouseholdAPI.updateHealthEvent(householdId: householdId, id: id, healthEventUpdateRequest: healthEventUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **healthEventUpdateRequest** | [**HealthEventUpdateRequest**](HealthEventUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**HealthEventResponse**](HealthEventResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateMedication**
```swift
    open class func updateMedication(householdId: Int, id: String, medicationUpdateRequest: MedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: MedicationResponse?, _ error: Error?) -> Void)
```

Update a medication.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let medicationUpdateRequest = MedicationUpdateRequest(medication: MedicationUpdateAttributes(name: "name_example", friendlyName: "friendlyName_example", barcode: "barcode_example", dmdCode: "dmdCode_example", dmdSystem: "dmdSystem_example", dmdConceptClass: "dmdConceptClass_example", category: "category_example", description: "description_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", currentSupply: "currentSupply_example", reorderThreshold: "reorderThreshold_example", warnings: "warnings_example", locationId: 123, defaultScheduleType: "defaultScheduleType_example")) // MedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a medication.
HouseholdAPI.updateMedication(householdId: householdId, id: id, medicationUpdateRequest: medicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **medicationUpdateRequest** | [**MedicationUpdateRequest**](MedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**MedicationResponse**](MedicationResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateNotificationPreference**
```swift
    open class func updateNotificationPreference(householdId: Int, notificationPreferenceUpdateRequest: NotificationPreferenceUpdateRequest, completion: @escaping (_ data: NotificationPreferenceResponse?, _ error: Error?) -> Void)
```

Update the signed-in person's notification preference.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let notificationPreferenceUpdateRequest = NotificationPreferenceUpdateRequest(notificationPreference: NotificationPreferenceAttributes(enabled: false, doseDueEnabled: false, missedDoseEnabled: false, lowStockEnabled: false, privateTextEnabled: false, morningTime: "morningTime_example", afternoonTime: "afternoonTime_example", eveningTime: "eveningTime_example", nightTime: "nightTime_example")) // NotificationPreferenceUpdateRequest | 

// Update the signed-in person's notification preference.
HouseholdAPI.updateNotificationPreference(householdId: householdId, notificationPreferenceUpdateRequest: notificationPreferenceUpdateRequest) { (response, error) in
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
 **notificationPreferenceUpdateRequest** | [**NotificationPreferenceUpdateRequest**](NotificationPreferenceUpdateRequest.md) |  | 

### Return type

[**NotificationPreferenceResponse**](NotificationPreferenceResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
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
HouseholdAPI.updatePerson(householdId: householdId, id: id, personUpdateRequest: personUpdateRequest) { (response, error) in
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

# **updatePersonMedication**
```swift
    open class func updatePersonMedication(householdId: Int, id: String, personMedicationUpdateRequest: PersonMedicationUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: PersonMedicationResponse?, _ error: Error?) -> Void)
```

Update a person medication assignment.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let personMedicationUpdateRequest = PersonMedicationUpdateRequest(personMedication: PersonMedicationAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", administrationKind: "administrationKind_example", notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example")) // PersonMedicationUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a person medication assignment.
HouseholdAPI.updatePersonMedication(householdId: householdId, id: id, personMedicationUpdateRequest: personMedicationUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **personMedicationUpdateRequest** | [**PersonMedicationUpdateRequest**](PersonMedicationUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**PersonMedicationResponse**](PersonMedicationResponse.md)

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
HouseholdAPI.updatePersonWithPut(householdId: householdId, id: id, personUpdateRequest: personUpdateRequest) { (response, error) in
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

# **updateSchedule**
```swift
    open class func updateSchedule(householdId: Int, id: String, scheduleUpdateRequest: ScheduleUpdateRequest, ifMatch: String? = nil, completion: @escaping (_ data: ScheduleResponse?, _ error: Error?) -> Void)
```

Update a schedule.

### Example
```swift
// The following code samples are still beta. For any issue, please report via http://github.com/OpenAPITools/openapi-generator/issues/new
import MedTrackerAPI

let householdId = 987 // Int | 
let id = "id_example" // String | 
let scheduleUpdateRequest = ScheduleUpdateRequest(schedule: ScheduleAttributes(personId: "personId_example", medicationId: "medicationId_example", sourceDosageOptionId: "sourceDosageOptionId_example", doseAmount: "doseAmount_example", doseUnit: "doseUnit_example", frequency: "frequency_example", startDate: Date(), endDate: Date(), notes: "notes_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", doseCycle: "doseCycle_example", scheduleType: "scheduleType_example", scheduleConfig: ScheduleConfig(times: ["times_example"], weekdays: ["weekdays_example"], dates: [Date()], asNeeded: false, taperSteps: [ScheduleTaperStep(startDate: Date(), endDate: Date(), amount: "amount_example", doseAmount: "doseAmount_example", unit: "unit_example", doseUnit: "doseUnit_example", maxDailyDoses: 123, minHoursBetweenDoses: "minHoursBetweenDoses_example", times: ["times_example"])]))) // ScheduleUpdateRequest | 
let ifMatch = "ifMatch_example" // String |  (optional)

// Update a schedule.
HouseholdAPI.updateSchedule(householdId: householdId, id: id, scheduleUpdateRequest: scheduleUpdateRequest, ifMatch: ifMatch) { (response, error) in
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
 **id** | **String** |  | 
 **scheduleUpdateRequest** | [**ScheduleUpdateRequest**](ScheduleUpdateRequest.md) |  | 
 **ifMatch** | **String** |  | [optional] 

### Return type

[**ScheduleResponse**](ScheduleResponse.md)

### Authorization

[bearerAuth](../README.md#bearerAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

