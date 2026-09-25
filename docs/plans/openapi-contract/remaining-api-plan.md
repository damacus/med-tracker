# Complete the remaining API operations

The active user goal is to complete the remaining 89 API operations. The exact
baseline is preserved in `remaining-89-baseline.json`, extracted from the
verified inventory at commit `863585e0`. Adding routes alone does not complete
this goal: each operation requires implemented behaviour and specification-led
runtime evidence for its requests, responses, access controls and failure paths.

## Execution

Continue the existing one-writer charter. The implementation agent owns tests,
handlers and its evidence report; the orchestrator independently reviews and
publishes verified checkpoints. Keep the original journey/UI workspace intact.

Start with dosage-option list, create, detail, PATCH and PUT. Then work through
related groups: session management and profiles; people; schedules and person
medication assignments; dose occurrences and pause periods; health events and
review prompts; notifications and device registration; household administration
and invitations; reports and external medication services; sync, exports and
imports. Adjust order for real dependencies without dropping any baseline item.

For each group, inspect the current OpenAPI operations and existing implementation,
write failing acceptance tests, implement the missing behaviour, review security
and correctness independently, and run the relevant isolated acceptance suite.
Use existing Task commands and Compose isolation. Keep concurrent test processes
inside their own project networks. Reuse common behaviour checks where they prove
the same contract, while retaining endpoint-specific assertions where needed.

Record unresolved specification decisions explicitly. Do not preserve discovered
bugs or silently add placeholder responses, permissive authorization, fabricated
medical values or simulated delivery of external side effects.

## Completion evidence

Track each baseline operation against the current operation map and test evidence.
Record route presence separately from proven behaviour. Shared helpers must have
direct tests, and endpoint wiring must have HTTP evidence. Publish scoped passing
checkpoints, but leave the goal active until all 89 baseline operations satisfy
their documented behaviour and required gates. No UI/PWA or memory-budget success
is implied by completion of this API goal.
