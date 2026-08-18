
# DosageOption

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int** |  |  |
| **portableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **medicationId** | **kotlin.Int** |  |  |
| **medicationPortableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **amount** | **kotlin.String** |  |  |
| **unit** | **kotlin.String** |  |  |
| **frequency** | **kotlin.String** |  |  |
| **description** | **kotlin.String** |  |  |
| **defaultForAdults** | **kotlin.Boolean** |  |  |
| **defaultForChildren** | **kotlin.Boolean** |  |  |
| **defaultMaxDailyDoses** | **kotlin.Int** |  |  |
| **defaultMinHoursBetweenDoses** | **kotlin.String** |  |  |
| **defaultDoseCycle** | [**inline**](#DefaultDoseCycle) |  |  |
| **currentSupply** | **kotlin.String** |  |  |
| **reorderThreshold** | **kotlin.String** |  |  |
| **updatedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |


<a id="DefaultDoseCycle"></a>
## Enum: default_dose_cycle
| Name | Value |
| ---- | ----- |
| defaultDoseCycle | daily, weekly, monthly |



