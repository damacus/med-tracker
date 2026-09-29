# Complete the remaining API operations

The active user goal is to complete all 118 documented API operations. The fixed
scope is preserved in `fixed-118-baseline.json`; the original 89-operation
baseline remains in `remaining-89-baseline.json` for comparison. Adding routes alone does not complete
this goal: each operation requires implemented behaviour and specification-led
runtime evidence for its requests, responses, access controls and failure paths.

## Execution

Use separate production and test ownership: Sol implements handlers and owns
shared integration files; Luna writes bounded tests, escalating complex diagnosis
to Sol. The orchestrator independently reviews and publishes verified checkpoints.
Keep the original journey/UI workspace intact.

Continue after household administration settings with medication workflows,
administration, personal records, data exchange and reports, then external
integrations. Verify already-present operations alongside each related family.
Adjust order for real dependencies without dropping any fixed-scope item.

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
checkpoints, but leave the goal active until all 118 documented operations satisfy
their documented behaviour and required gates. No UI/PWA or memory-budget success
is implied by completion of this API goal.
