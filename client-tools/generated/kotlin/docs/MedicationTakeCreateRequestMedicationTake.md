
# MedicationTakeCreateRequestMedicationTake

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **sourceType** | [**inline**](#SourceType) |  |  |
| **sourceId** | **kotlin.String** |  |  |
| **takenAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **clientUuid** | [**java.util.UUID**](java.util.UUID.md) |  |  [optional] |
| **doseAmount** | **kotlin.String** |  |  [optional] |
| **doseUnit** | **kotlin.String** |  |  [optional] |
| **takenFromMedicationId** | **kotlin.Int** |  |  [optional] |


<a id="SourceType"></a>
## Enum: source_type
| Name | Value |
| ---- | ----- |
| sourceType | schedule, person_medication |



