
# AiMedicationDoseSuggestion

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **amount** | **kotlin.String** |  |  |
| **unit** | [**inline**](#Unit) |  |  |
| **defaultMaxDailyDoses** | **kotlin.Int** |  |  |
| **defaultMinHoursBetweenDoses** | **kotlin.String** |  |  |
| **defaultDoseCycle** | [**inline**](#DefaultDoseCycle) |  |  |
| **evidence** | [**AiMedicationDoseEvidence**](AiMedicationDoseEvidence.md) |  |  |
| **description** | **kotlin.String** |  |  [optional] |


<a id="Unit"></a>
## Enum: unit
| Name | Value |
| ---- | ----- |
| unit | tablet, capsule, gummy, mg, ml, g, mcg, IU, spray, drop, sachet, pad |


<a id="DefaultDoseCycle"></a>
## Enum: default_dose_cycle
| Name | Value |
| ---- | ----- |
| defaultDoseCycle | daily, weekly, monthly |



