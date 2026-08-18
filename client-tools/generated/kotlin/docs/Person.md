
# Person

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int** |  |  |
| **portableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **updatedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **name** | **kotlin.String** |  |  |
| **email** | **kotlin.String** |  |  |
| **dateOfBirth** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **personType** | [**inline**](#PersonType) |  |  |
| **hasCapacity** | **kotlin.Boolean** |  |  |
| **age** | **kotlin.Int** |  |  |
| **locationIds** | **kotlin.collections.List&lt;kotlin.Int&gt;** |  |  |
| **locationPortableIds** | [**kotlin.collections.List&lt;java.util.UUID&gt;**](java.util.UUID.md) |  |  |
| **notificationPreferenceId** | **kotlin.Int** |  |  |
| **notificationPreferencePortableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |


<a id="PersonType"></a>
## Enum: person_type
| Name | Value |
| ---- | ----- |
| personType | adult, minor, dependent_adult |



