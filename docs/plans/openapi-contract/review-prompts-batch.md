# Medication review prompts

Implement listMedicationReviewPrompts, getMedicationReviewPrompt,
updateMedicationReviewPrompt and updateMedicationReviewPromptWithPut against
the fixed 118-operation scope.

Preserve the Rails list's on-demand creation of missing snapshots from stored
detectable evidence and active medication pairs. Repeated listing must not
duplicate prompts or refresh immutable snapshots. Port curated and automatic
ingredient/class matching, terminology aliases and instruction classification;
use deterministic stored evidence, without claiming live-provider verification.

Correct a discovered classifier bug: a benign no-adjustment sentence must not
suppress an explicit contraindication, avoidance or monitoring instruction in
another matched sentence. Retain exclusion for genuinely no-action-only
evidence, and test the conflicting-sentence case before implementing the fix.

Active adult household members can read prompts for people they can view;
updates require current manage access. Preserve hidden and foreign-record
boundaries. Cover queue filters, pagination, exact response shapes, decimal
string record IDs, nullable decimal-string reviewer membership IDs and no-store.

PATCH and PUT accept only the documented review fields. Require the current
If-Match (428 missing, 409 stale), validate review statuses and practitioner
details, and reject invalid input without mutation. Unknown snapshot fields
must be rejected under the strict schema, correcting older tests that expected
them to be silently ignored. Evidence snapshots remain immutable.

Successful updates record medication_review_prompt.updated with the person and
prompt IDs and status transition, without practitioner names or review notes.
Reuse shared idempotency with fresh authorization and endpoint-specific proof.

Luna owns the isolated acceptance file; primary Sol owns production and shared
integration. Compile initial tests and show HTTP RED before production edits;
expand remaining tests in parallel. Independent review and isolated acceptance
are required before publication and evidence credit.
