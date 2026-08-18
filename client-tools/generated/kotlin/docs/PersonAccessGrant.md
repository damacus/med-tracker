
# PersonAccessGrant

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **id** | **kotlin.Int** |  |  |
| **householdMembershipId** | **kotlin.Int** |  |  |
| **personId** | **kotlin.Int** |  |  |
| **personName** | **kotlin.String** |  |  |
| **accessLevel** | [**inline**](#AccessLevel) |  |  |
| **relationshipType** | [**inline**](#RelationshipType) |  |  |
| **expiresAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |
| **revokedAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  |


<a id="AccessLevel"></a>
## Enum: access_level
| Name | Value |
| ---- | ----- |
| accessLevel | view, record, manage |


<a id="RelationshipType"></a>
## Enum: relationship_type
| Name | Value |
| ---- | ----- |
| relationshipType | self, parent, family_member, carer, professional |



