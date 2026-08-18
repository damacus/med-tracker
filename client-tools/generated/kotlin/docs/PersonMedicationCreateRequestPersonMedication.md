
# PersonMedicationCreateRequestPersonMedication

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **personId** | **kotlin.String** |  |  |
| **medicationId** | **kotlin.String** |  |  |
| **sourceDosageOptionId** | **kotlin.String** |  |  [optional] |
| **doseAmount** | **kotlin.String** |  |  [optional] |
| **doseUnit** | **kotlin.String** |  |  [optional] |
| **administrationKind** | [**inline**](#AdministrationKind) |  |  [optional] |
| **notes** | **kotlin.String** |  |  [optional] |
| **maxDailyDoses** | **kotlin.Int** |  |  [optional] |
| **minHoursBetweenDoses** | **kotlin.String** |  |  [optional] |
| **doseCycle** | [**inline**](#DoseCycle) |  |  [optional] |


<a id="AdministrationKind"></a>
## Enum: administration_kind
| Name | Value |
| ---- | ----- |
| administrationKind | routine, as_needed |


<a id="DoseCycle"></a>
## Enum: dose_cycle
| Name | Value |
| ---- | ----- |
| doseCycle | daily, weekly, monthly |



