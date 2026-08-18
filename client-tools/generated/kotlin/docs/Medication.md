
# Medication

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int** |  |  |
| **portableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **name** | **kotlin.String** |  |  |
| **displayName** | **kotlin.String** |  |  |
| **category** | **kotlin.String** |  |  |
| **description** | **kotlin.String** |  |  |
| **doseAmount** | **kotlin.String** |  |  |
| **doseUnit** | **kotlin.String** |  |  |
| **currentSupply** | **kotlin.String** |  |  |
| **reorderThreshold** | **kotlin.String** |  |  |
| **reorderStatus** | [**inline**](#ReorderStatus) |  |  |
| **locationId** | **kotlin.Int** |  |  |
| **locationPortableId** | [**java.util.UUID**](java.util.UUID.md) |  |  |
| **updatedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **lowStock** | **kotlin.Boolean** |  |  |
| **outOfStock** | **kotlin.Boolean** |  |  |
| **daysUntilLowStock** | **kotlin.Int** |  |  |
| **daysUntilOutOfStock** | **kotlin.Int** |  |  |


<a id="ReorderStatus"></a>
## Enum: reorder_status
| Name | Value |
| ---- | ----- |
| reorderStatus | ordered, received |



