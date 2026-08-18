
# SyncBatchOperation

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **action** | [**inline**](#Action) |  |  |
| **resourceType** | [**inline**](#ResourceType) |  |  |
| **id** | **kotlin.String** |  |  [optional] |
| **ifMatch** | **kotlin.String** | Required for update and delete operations. Use the exact latest ETag. |  [optional] |
| **attributes** | [**kotlin.collections.Map&lt;kotlin.String, kotlin.Any&gt;**](kotlin.Any.md) | Fields accepted by the selected resource and action. |  [optional] |


<a id="Action"></a>
## Enum: action
| Name | Value |
| ---- | ----- |
| action | create, update, delete |


<a id="ResourceType"></a>
## Enum: resource_type
| Name | Value |
| ---- | ----- |
| resourceType | medication, health_event, medication_take |



