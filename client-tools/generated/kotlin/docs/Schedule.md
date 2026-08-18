
# Schedule

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
| **frequency** | **kotlin.String** |  |  |
| **doseCycle** | [**inline**](#DoseCycle) |  |  |
| **startDate** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **endDate** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **active** | **kotlin.Boolean** |  |  |
| **paused** | **kotlin.Boolean** |  |  |
| **notes** | **kotlin.String** |  |  |
| **updatedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **maxDailyDoses** | **kotlin.Int** |  |  |
| **minHoursBetweenDoses** | **kotlin.String** |  |  |


<a id="DoseCycle"></a>
## Enum: dose_cycle
| Name | Value |
| ---- | ----- |
| doseCycle | daily, weekly, monthly |



