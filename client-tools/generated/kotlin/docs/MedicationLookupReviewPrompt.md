
# MedicationLookupReviewPrompt

## Properties
| Name | Type | Description | Notes |
| ------------ | ------------- | ------------- | ------------- |
| **evidenceRecordId** | **kotlin.Int** |  |  |
| **riskLevel** | [**inline**](#RiskLevel) |  |  |
| **riskLevelLabel** | **kotlin.String** |  |  |
| **matchConfidence** | [**inline**](#MatchConfidence) |  |  |
| **matchConfidenceLabel** | **kotlin.String** |  |  |
| **matchedTerm** | **kotlin.String** |  |  |
| **matchType** | [**inline**](#MatchType) |  |  |
| **sourceInstruction** | **kotlin.String** |  |  |
| **matchReason** | **kotlin.String** |  |  |
| **interactingMedicationName** | **kotlin.String** |  |  |
| **description** | **kotlin.String** |  |  |
| **sourceName** | **kotlin.String** |  |  |
| **sourceCheckedOn** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **sourceVersion** | **kotlin.String** |  |  |
| **sourceEffectiveOn** | [**java.time.LocalDate**](java.time.LocalDate.md) |  |  |
| **sourceUrl** | [**java.net.URI**](java.net.URI.md) |  |  |
| **evidenceText** | **kotlin.String** |  |  |


<a id="RiskLevel"></a>
## Enum: risk_level
| Name | Value |
| ---- | ----- |
| riskLevel | high, moderate, low, unknown |


<a id="MatchConfidence"></a>
## Enum: match_confidence
| Name | Value |
| ---- | ----- |
| matchConfidence | high, moderate, low, unknown |


<a id="MatchType"></a>
## Enum: match_type
| Name | Value |
| ---- | ----- |
| matchType | curated, ingredient, class |



