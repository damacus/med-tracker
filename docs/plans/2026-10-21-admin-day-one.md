# Administration expansion

Approved owner decisions, 21 October 2026. Issue #2707 remains open until its accepted scope is delivered. Each complete vertical slice is a separate PR against main. Unmerged dependencies are named with a focused parent comparison; no stack tooling changes or GitHub merges are authorised.

## Shared boundaries

Rust/Loco is primary; Rails is a read-only visual reference. Use canonical membership operations, restricted runtime credentials, RLS, transactional audit, last-owner protection, CSRF and live authority checks. Keep account sign-in separate from household membership. Synthetic disposable databases only. No live data or deployment. One source writer per slice; controller owns final CI/docs, independent security review and publication. Preserve all five locales, query state, keyboard access, errors and focus. SQL performs bounded filtering and sorting. Use Fish, RTK and repository Task commands, with failing behavioural evidence before executable changes.

## Slice 1: finish and integrate the member directory

Rebase PR #2846 onto current main, preserving main changes and the original branch under a backup ref. Integrate PR #2850 with a history-preserving local merge; leave that GitHub PR open. Retain role editing and active/suspended/revoked status, confirmations, search, role/status filters, name/email/role/status sorting, page size 25, deterministic ordering, pagination and language state.

Fix inactive accountless identities with a narrowly authorised household directory projection returning only membership identity, name and email. Use the same projection for list/search/sort/editor; do not broaden direct people or clinical access. Preserve canonical account identity precedence. Prove restricted-runtime denials for ordinary members, foreign household and stale authority. When a submitted action loses authority, return forbidden and do not mutate; directory rerendering is unavailable because it independently rechecks authority.

Compare synthetic screenshots at 1440, 390 and 320 pixels against retained shared-layout and private Rails references. Source owner runs focused request/browser Tasks and supplies exact RED/GREEN evidence. Controller runs final task ci and task docs:build after source freezes, then independent security review and publication.

## Slice 2: member details and history

Provide the complete member detail/history journey using the same scoped identity and permissions. Preserve immutable attribution and distinguish membership history from sign-in state. Define and test authorised history, empty/error states, navigation and mobile behaviour before implementation. Controller owns this subsequent slice.

## Slice 3: invitation management and delivery attempts

Provide searchable and filtered invitation management. Track queued, sent-to-provider and failed delivery attempts through a dedicated invitation worker with transactional enqueue. Sent-to-provider does not claim recipient delivery. Retain live permission rechecks, expiry/revocation/replay handling, atomic domain writes and audit. Test queue rollback and worker failures, plus accessible management controls. Controller owns this subsequent slice.

## Slice 4: bulk suspend and revoke

Allow selection on the current page only, at most 25 members. Apply per-member changes with partial success and clear individual results. Process the actor last and stop if authority is lost. Preserve last-active-owner safeguards, CSRF, live checks, canonical operations and audit for each accepted mutation. Test mixed success/failure, stale authority, actor ordering and mobile/keyboard interaction. Controller owns this subsequent slice.

## Exclusions and evidence

Direct administrator account creation, other-account profile/password editing, guardian recovery, generic care-relationship changes, platform/dm+d work and production deployment remain outside these slices. Prior care-relationship exploration is preserved privately. Existing Cedar delegate_person denial must not be bypassed.

Routing: scoped implementation and routine review use Sol; independent security review uses Astra under controller ownership. Serena is unavailable, so use direct repository inspection. Current route retains the existing owner; projection security requires independent review before acceptance.

## Slice 1 focused evidence

The accountless identity regression first failed through App::boot with restricted runtime credentials: searching ACCOUNTLESS returned no memberships instead of the expected revoked/suspended pair. The final focused request Task passed all four member-administration tests. The new projection catalogue test verifies SECURITY DEFINER, search_path=pg_catalog, the three-column identity result and execute grants; runtime tests retain direct people invisibility and reject foreign scope, ordinary role, revoked actor, lockout and inactive household lifecycle. Rejected stale-manager posts return 403 without directory links or changes to target state/audit count.

The synthetic browser journey covers directory/confirmation and accountless role editing at 1440, 390 and 320 pixels, including horizontal-overflow checks. Evidence lives in docs/screenshots/browser-journeys/members-{desktop,mobile,narrow}.png and matching -role.png files. The shared shell, rounded filter/table panels, typography and responsive role form are compared against docs/screenshots/shared-layout and the retained private Rails member reference. The added filter/sort/status/action controls deliberately extend the shared-layout reference. No sign-in activation or platform-administrator controls are copied from Rails.

Final CI/docs and independent security acceptance remain controller gates. Local focused tests do not claim publication, merge or deployment.
