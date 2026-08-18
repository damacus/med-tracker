
# MedicationUpdateAttributes

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **name** | **kotlin.String** |  |  [optional] |
| **friendlyName** | **kotlin.String** |  |  [optional] |
| **barcode** | **kotlin.String** |  |  [optional] |
| **dmdCode** | **kotlin.String** |  |  [optional] |
| **dmdSystem** | **kotlin.String** |  |  [optional] |
| **dmdConceptClass** | **kotlin.String** |  |  [optional] |
| **category** | **kotlin.String** |  |  [optional] |
| **description** | **kotlin.String** |  |  [optional] |
| **doseAmount** | **kotlin.String** |  |  [optional] |
| **doseUnit** | [**inline**](#DoseUnit) |  |  [optional] |
| **currentSupply** | **kotlin.String** |  |  [optional] |
| **reorderThreshold** | **kotlin.String** |  |  [optional] |
| **warnings** | **kotlin.String** |  |  [optional] |
| **locationId** | **kotlin.Int** |  |  [optional] |
| **defaultScheduleType** | [**inline**](#DefaultScheduleType) |  |  [optional] |


<a id="DoseUnit"></a>
## Enum: dose_unit
| Name | Value |
| ---- | ----- |
| doseUnit | tablet, capsule, gummy, mg, ml, g, mcg, IU, spray, drop, sachet, pad |


<a id="DefaultScheduleType"></a>
## Enum: default_schedule_type
| Name | Value |
| ---- | ----- |
| defaultScheduleType | daily, multiple_daily, weekly, specific_dates, prn, tapering, every_other_day |



