
# ScheduleAttributes

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **personId** | **kotlin.String** |  |  [optional] |
| **medicationId** | **kotlin.String** |  |  [optional] |
| **sourceDosageOptionId** | **kotlin.String** |  |  [optional] |
| **doseAmount** | **kotlin.String** |  |  [optional] |
| **doseUnit** | **kotlin.String** |  |  [optional] |
| **frequency** | **kotlin.String** |  |  [optional] |
| **startDate** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  [optional] |
| **endDate** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  [optional] |
| **notes** | **kotlin.String** |  |  [optional] |
| **maxDailyDoses** | **kotlin.Int** |  |  [optional] |
| **minHoursBetweenDoses** | **kotlin.String** |  |  [optional] |
| **doseCycle** | [**inline**](#DoseCycle) |  |  [optional] |
| **scheduleType** | [**inline**](#ScheduleType) |  |  [optional] |
| **scheduleConfig** | [**ScheduleConfig**](ScheduleConfig.md) |  |  [optional] |


<a id="DoseCycle"></a>
## Enum: dose_cycle
| Name | Value |
| ---- | ----- |
| doseCycle | daily, weekly, monthly |


<a id="ScheduleType"></a>
## Enum: schedule_type
| Name | Value |
| ---- | ----- |
| scheduleType | daily, multiple_daily, weekly, specific_dates, prn, tapering, every_other_day |



