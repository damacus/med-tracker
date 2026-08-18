
# PersonAccessGrantAttributes

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **householdMembershipId** | **kotlin.Int** |  |  |
| **personId** | **kotlin.Int** |  |  |
| **accessLevel** | [**inline**](#AccessLevel) |  |  |
| **relationshipType** | [**inline**](#RelationshipType) |  |  |
| **expiresAt** | [**java.time.OffsetDateTime**](java.time.OffsetDateTime.md) |  |  [optional] |


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



