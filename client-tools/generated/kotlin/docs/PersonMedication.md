
# PersonMedication

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int** |  |  |
| **portableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **personId** | **kotlin.Int** |  |  |
| **personPortableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **medicationId** | **kotlin.Int** |  |  |
| **medicationPortableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **doseAmount** | **kotlin.String** |  |  |
| **doseUnit** | **kotlin.String** |  |  |
| **active** | **kotlin.Boolean** |  |  |
| **paused** | **kotlin.Boolean** |  |  |
| **doseCycle** | [**inline**](#DoseCycle) |  |  |
| **administrationKind** | [**inline**](#AdministrationKind) |  |  |
| **notes** | **kotlin.String** |  |  |
| **position** | **kotlin.Int** |  |  |
| **updatedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **maxDailyDoses** | **kotlin.Int** |  |  |
| **minHoursBetweenDoses** | **kotlin.String** |  |  |


<a id="DoseCycle"></a>
## Enum: dose_cycle
| Name | Value |
| ---- | ----- |
| doseCycle | daily, weekly, monthly |


<a id="AdministrationKind"></a>
## Enum: administration_kind
| Name | Value |
| ---- | ----- |
| administrationKind | routine, as_needed |



