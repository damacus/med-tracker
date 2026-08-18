
# HealthEvent

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int** |  |  |
| **portableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **personId** | **kotlin.Int** |  |  |
| **personPortableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **eventKind** | [**inline**](#EventKind) |  |  |
| **severity** | [**inline**](#Severity) |  |  |
| **title** | **kotlin.String** |  |  |
| **notes** | **kotlin.String** |  |  |
| **startedOn** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **endedOn** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **updatedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **medicationIds** | **kotlin.collections.List&lt;kotlin.Int&gt;** |  |  |
| **medicationPortableIds** | [**kotlin.collections.List&lt;java.util.UUID&gt;**](java.util.UUID.md) |  |  |


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



