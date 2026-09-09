# Mobile API parity verification

This change delivers the six API gaps through [GitHub stack #2155](https://github.com/damacus/med-tracker/stack/2155). [Issue #2152](https://github.com/damacus/med-tracker/issues/2152) remains the review and landing tracker. Merging and deployment require separate approval.

| Area | Implementation | Evidence |
| --- | --- | --- |
| Dose outcomes and corrections | #2154–#2190, in stack order | Bounded schedule/routine reads, not-taken/reopen/take writes, current grants, ETags, competing outcomes, retained history, reports, reminders, sync and portable round trips. Both existing outcome plans have no unchecked implementation tasks. |
| Stock removal | #2191 | Decimal validation, source access, insufficient stock, UUID replay, ordered history and no clinical take. |
| Locations and person memberships | #2192 | Household scope, management grants, ETag conflicts, retained administration history and duplicate membership prevention. |
| Reviews and protected reports | #2193–#2194 | Immutable evidence, practitioner attribution, review versions, selected-person access, date bounds, JSON/PDF export audit and renderer failure. |
| Profile, avatar and invitations | #2195–#2200 | Atomic profile preferences, protected image storage, canonical identity, tenant/runtime authentication context, verified invitation acceptance and fresh-MFA resend. |
| Offline writes | #2201–#2204 | Care and inventory replay, pause/resume/reorder, mixed-batch rollback, current authority on cached results and the explicit supported-operation matrix. |

Numbers in the table denote the stack's relevant layers, not a request to merge unrelated intervening PRs. #2178 is not in this stack.

## Checks

Development used focused Rails specs for each layer. The final offline integration selection passed 247 examples; subsequent coverage, restack and replay regressions passed their focused checks. Required RuboCop, strict OpenSpec validation, documentation and OpenAPI route/schema checks passed. Root Swift and Kotlin clients generated deterministically and compiled successfully.

GitHub's commit-matched CI is the full Rails verification authority. Older cancelled attempts can remain visible in a PR's summary alongside a newer successful run; compare the run's head SHA and conclusion before assessing the layer.

The final replay repair passed 65 focused examples. [CI run 34324970074](https://github.com/damacus/med-tracker/actions/runs/34324970074) then confirmed 724 of 799 API branches covered (90.61%), above the unchanged 90% gate. The added checks cover current authority for cached results, missing-record refusal, shared review version errors and atomic rollback.

Android's explicit OpenAPI pin in #2205 matches the root contract and records source revision `f7b1a177469137e24f698d9322bae0e9fc9cd234` and its checksum in `mobile/android/OPENAPI_PROVENANCE.md`. `task android:ci` passed the phone, Wear and protocol checks, lint, assembly, release security, Wear dependency boundary and generated-client drift checks. The pin uses the existing release/password client split. There is no imported iOS build root in this checkout; root Swift contract generation was verified without creating another iOS import.

## Integration with the merged pause stack

The stack was rebased onto `44137dc20103dbf43dfea8822d040933745ef9a5`, which includes #2150. Conflict resolutions retain pause history alongside dose outcomes in portable v2 exports, imports and sync snapshots. Both existing export selectors remain accepted. Cached pause and batch responses retain their current-authority checks, and the offline operation catalog includes pause-period create and close.

The focused integration run covered 189 examples. After correcting the combined capability expectation, the final contract and regression selection passed 150 examples. RuboCop passed across 1,965 files. Android CI passed after regenerating the combined contract; the final capability enum refresh also passed client drift verification. Android provenance now records root contract revision `20db1ac3f94b34eb94e40a5efb0a95ebfd9367f6`.

These are focused local checks. The earlier full-suite browser failure and the CI evidence above predate this rebase; fresh CI on the published branch heads remains the merge gate.

## Review boundaries

- API and contract delivery is complete; new native screens are separate work.
- Identity, profile, avatar, invitation, membership/access changes and reports remain online-only. Creating a care person uses the existing grant workflow and can require session renewal before a lost-response retry.
- Queued pauses start at server replay time. Their current ETags and batch keys protect later changes and retries.
- No deployment, canary activation or merge occurred during this work.
