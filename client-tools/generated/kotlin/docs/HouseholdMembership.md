
# HouseholdMembership

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int** |  |  |
| **accountId** | **kotlin.Int** |  |  |
| **email** | **kotlin.String** |  |  |
| **personId** | **kotlin.Int** |  |  |
| **personName** | **kotlin.String** |  |  |
| **role** | [**inline**](#Role) |  |  |
| **status** | [**inline**](#Status) |  |  |
| **permissionsVersion** | **kotlin.Int** |  |  |
| **joinedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |


<a id="Role"></a>
## Enum: role
| Name | Value |
| ---- | ----- |
| role | owner, administrator, member |


<a id="Status"></a>
## Enum: status
| Name | Value |
| ---- | ----- |
| status | active, suspended, revoked |



