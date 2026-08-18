
# SyncBatchResult

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **index** | **kotlin.Int** |  |  |
| **action** | [**inline**](#Action) |  |  |
| **recordType** | [**inline**](#RecordType) |  |  |
| **recordPortableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **etag** | **kotlin.String** |  |  [optional] |
| **replayed** | **kotlin.Boolean** |  |  [optional] |


<a id="Action"></a>
## Enum: action
| Name | Value |
| ---- | ----- |
| action | create, update, delete |


<a id="RecordType"></a>
## Enum: record_type
| Name | Value |
| ---- | ----- |
| recordType | Medication, HealthEvent, MedicationTake |



