
# HealthEventCreateRequestHealthEvent

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **personId** | **kotlin.String** |  |  |
| **eventKind** | [**inline**](#EventKind) |  |  |
| **title** | **kotlin.String** |  |  |
| **startedOn** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **severity** | [**inline**](#Severity) |  |  [optional] |
| **notes** | **kotlin.String** |  |  [optional] |
| **endedOn** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  [optional] |
| **medicationIds** | **kotlin.collections.Set&lt;kotlin.String&gt;** |  |  [optional] |


<a id="EventKind"></a>
## Enum: event_kind
| Name | Value |
| ---- | ----- |
| eventKind | illness, suspected_side_effect |


<a id="Severity"></a>
## Enum: severity
| Name | Value |
| ---- | ----- |
| severity | mild, moderate, severe |



