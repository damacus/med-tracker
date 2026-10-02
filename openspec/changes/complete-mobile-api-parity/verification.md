# Mobile API parity verification

This change delivers the six API gaps through one consolidated PR against `main`. The original 90 commits from [GitHub stack #2155](https://github.com/damacus/med-tracker/stack/2155) remain in the branch history. The stack links below retain the implementation review trail. [Issue #2152](https://github.com/damacus/med-tracker/issues/2152) remains the review and landing tracker. Merging and deployment require separate approval.

| Area | Implementation | Evidence |
| --- | --- | --- |
| Dose outcomes and corrections | #2154–#2190, in stack order | Bounded schedule/routine reads, not-taken/reopen/take writes, current grants, ETags, competing outcomes, retained history, reports, reminders, sync and portable round trips. Both existing outcome plans have no unchecked implementation tasks. |
| Stock removal | #2191 | Decimal validation, source access, insufficient stock, UUID replay, ordered history and no clinical take. |
| Locations and person memberships | #2192 | Household scope, management grants, ETag conflicts, retained administration history and duplicate membership prevention. |
| Reviews and protected reports | #2193–#2194 | Immutable evidence, practitioner attribution, review versions, selected-person access, date bounds, JSON/PDF export audit and renderer failure. |
| Profile, avatar and invitations | #2195–#2200 | Atomic profile preferences, protected image storage, canonical identity, tenant/runtime authentication context, verified invitation acceptance and fresh-MFA resend. |
| Offline writes | #2201–#2204 | Care and inventory replay, pause/resume/reorder, mixed-batch rollback, current authority on cached results and the explicit supported-operation matrix. |

Numbers in the table identify the original implementation PRs. They are superseded by the consolidated delivery PR and are not separate merge targets. #2178 is not part of this delivery.

## Checks

Development used focused Rails specs for each layer. The final offline integration selection passed 247 examples; subsequent coverage, restack and replay regressions passed their focused checks. Required RuboCop, strict OpenSpec validation, documentation and OpenAPI route/schema checks passed. Root Swift and Kotlin clients generated deterministically and compiled successfully.

The complete consolidated revision must pass local `task test` before each push. GitHub CI remains the merge gate for that same revision. Older cancelled attempts are not verification evidence.

The final replay repair passed 65 focused examples. [CI run 34324970074](https://github.com/damacus/med-tracker/actions/runs/34324970074) then confirmed 724 of 799 API branches covered (90.61%), above the unchanged 90% gate. The added checks cover current authority for cached results, missing-record refusal, shared review version errors and atomic rollback.

Android's explicit OpenAPI pin in #2205 matches the root contract and records source revision `f7b1a177469137e24f698d9322bae0e9fc9cd234` and its checksum in `mobile/android/OPENAPI_PROVENANCE.md`. `task android:ci` passed the phone, Wear and protocol checks, lint, assembly, release security, Wear dependency boundary and generated-client drift checks. The pin uses the existing release/password client split. There is no imported iOS build root in this checkout; root Swift contract generation was verified without creating another iOS import.

## Integration with the merged pause stack

The stack was rebased onto `44137dc20103dbf43dfea8822d040933745ef9a5`, which includes #2150. Conflict resolutions retain pause history alongside dose outcomes in portable v2 exports, imports and sync snapshots. Both existing export selectors remain accepted. Cached pause and batch responses retain their current-authority checks, and the offline operation catalog includes pause-period create and close.

The focused integration run covered 189 examples. After correcting the combined capability expectation, the final contract and regression selection passed 150 examples. RuboCop passed across 1,965 files. Android CI passed after regenerating the combined contract; the final capability enum refresh also passed client drift verification. Android provenance now records root contract revision `20db1ac3f94b34eb94e40a5efb0a95ebfd9367f6`.

These integration checks were focused local runs. The CI evidence above predates the rebase and does not establish the consolidated revision's full-suite result.

## Consolidated delivery and release messages

The subsequent full-suite sweep passed the first eleven stack layers. It exposed a keyboard-dialog readiness race and a dashboard regression whose household membership setup collided with full-suite fixtures. Both repairs passed their affected specs, RuboCop and full-suite reruns, and are included in the consolidated branch.

The remaining delivery is verified on the complete combined revision rather than generating another CI run for every superseded layer. The consolidated PR records the final local full-suite result and its exact commit. The original PR branches remain available for history.

Squash merge the consolidated PR. Its `BEGIN_COMMIT_OVERRIDE` block preserves every original commit message as a release-please footer, so separate features and fixes remain separate changelog entries. Rebase merging can repeat those entries across the associated commits. Maintenance prefixes retain the repository's configured changelog visibility.

## Review boundaries

- API and contract delivery is complete; new native screens are separate work.
- Identity, profile, avatar, invitation, membership/access changes and reports remain online-only. Creating a care person uses the existing grant workflow and can require session renewal before a lost-response retry.
- Queued pauses start at server replay time. Their current ETags and batch keys protect later changes and retries.
- No deployment, canary activation or merge occurred during this work.
