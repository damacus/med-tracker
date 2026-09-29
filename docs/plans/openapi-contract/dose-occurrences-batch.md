# Dose occurrence outcomes

After pause history, implement eight operations: listScheduleDoseOccurrences,
recordScheduleDoseNotTaken, reopenScheduleDoseOccurrence,
takeScheduleDoseOccurrence, listPersonMedicationDoseOccurrences,
recordPersonMedicationDoseNotTaken, reopenPersonMedicationDoseOccurrence and
takePersonMedicationDoseOccurrence.

Use the fixed OpenAPI schemas, Rails OccurrenceProjection/OccurrenceResolver
and the existing doses.rs scenarios as reference. Lists project stable opaque
keys over at most 31 inclusive dates without persisting outcomes. Preserve
view/manage/take permission differences and hidden-source boundaries. Routine
and scheduled windows differ from as-needed sources; paused and retired state
must be respected.

Not-taken decisions retain reason, note and actor without altering stock.
Reopen requires the current ETag and clears decision metadata. Taking an open
occurrence uses normal administration validation; replacing a not-taken outcome
requires its current ETag. Take insertion, stock decrement and occurrence
transition must share one transaction. Reuse existing administration helpers
rather than implementing a second medication-safety rule set.

Matching client UUID and administration content replay without another stock
decrement. A changed administration payload with the same UUID conflicts; do
not copy Rails' UUID-only shortcut that silently ignores changed dose content.
Document this correction explicitly before deriving its tests. Preserve prior
outcome history and current authority checks on replay.

Occurrence keys must remain stable across restarts using a durable configured
signing secret. Preserve existing outcome rows through their source, window and
position identity. Rails MessageVerifier transport compatibility is not yet
proved; clients must refetch opaque keys after cutover and reauthentication
unless that compatibility is separately established. Do not discard outcome
history or generate a new random signing secret on every process start.

Compile initial tests and prove missing-route RED, expand remaining tests while
implementing, independently review, then run isolated acceptance and applicable
checks before publication and evidence credit. Sol owns production and shared
integration; Luna owns a separate bounded test file. No UI or deployment work.
