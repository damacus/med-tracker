# Administration expansion

Approved owner decisions, 21 October 2026. Issue #2707 remains open until its accepted scope is delivered. Each complete vertical slice is a separate PR against main. Unmerged dependencies are named with a focused parent comparison; no stack tooling changes or GitHub merges are authorised.

## Shared boundaries

Rust/Loco is primary; Rails is a read-only visual reference. Use canonical membership operations, restricted runtime credentials, RLS, transactional audit, last-owner protection, CSRF and live authority checks. Keep account sign-in separate from household membership. Synthetic disposable databases only. No live data or deployment. One source writer per slice; controller owns final CI/docs, independent security review and publication. Preserve all five locales, query state, keyboard access, errors and focus. SQL performs bounded filtering and sorting. Use Fish, RTK and repository Task commands, with failing behavioural evidence before executable changes.

## Slice 1: finish and integrate the member directory

Rebase PR #2846 onto current main, preserving main changes and the original branch under a backup ref. Integrate PR #2850 with a history-preserving local merge; leave that GitHub PR open. Retain role editing and active/suspended/revoked status, confirmations, search, role/status filters, name/email/role/status sorting, page size 25, deterministic ordering, pagination and language state.

Fix inactive accountless identities with a narrowly authorised household directory projection returning only membership identity, name and email. Use the same projection for list/search/sort/editor; do not broaden direct people or clinical access. Preserve canonical account identity precedence. Prove restricted-runtime denials for ordinary members, foreign household and stale authority. When a submitted action loses authority, return forbidden and do not mutate; directory rerendering is unavailable because it independently rechecks authority.

Compare synthetic screenshots at 1440, 390 and 320 pixels against retained shared-layout and private Rails references. Source owner runs focused request/browser Tasks and supplies exact RED/GREEN evidence. Controller runs final task ci and task docs:build after source freezes, then independent security review and publication.

## Slice 2: member details and history

In progress on codex/feat-member-details from verified slice1 revision684fd2ee1. One source writer owns this checkout; no source changes to slice1 checkout. Use scoped identity and canonical role/lifecycle commands without changing lock order. Member names link to a detail page with identity, role/status, role powers, clear suspend/revoke/restore effects and explicit distinction from global sign-in and guardianship. Compact visible directory actions retain member-specific accessible names. Preserve original list search/role/status/sort/direction/page/language through detail, role editor and successful/rejected actions.

History is a dedicated typed SQL projection scoped to current household and exact numeric metadata.target_membership_id, restricted to household_membership.role_updated, household_access.membership_changed and household_access.membership_created. Include rejected/no-change attempts with explicit outcome; never claim they changed membership. Page25 newest first with deterministic timestamp/id cursor bounded to the selected member. Reuse safe role/status/outcome allowlists and immutable actor references. Batch same-household actor names through narrow directory projection; fall back to original actor IDs without global identity lookup or raw clinical metadata. Reads add no new audit event.

Acceptance uses restricted full-App requests for member/missing/foreign/ordinary/revoked/locked scope, exact timeline filtering, escaped malicious metadata/name, safe actor fallback, cursor boundaries and navigation through actions. Browser keyboard/action/back links and1440/390/320 screenshots include dark, validation error and empty history. Controller owns final taskci/docs and independent review after source freeze. Slices3–4 remain unimplemented.

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

## Slice 2 focused evidence

Implemented member detail/history and compact accessible directory actions on the verified slice1/focus-fix base. Detail requests first failed404 before production changes. Restricted App::boot member request coverage now includes exact household/target history filtering,25-entry cursor/tie-break pages, actor-name escaping/fallback, read audit invariance, missing/foreign/ordinary/revoked/locked cases and preserved list/language context through lifecycle/role/error responses. Rejected or failed requested deltas explicitly say they were not applied; unchanged pairs are omitted. Dates reuse the existing translated date helper and authenticated account timezone, with ISO time attributes.

Focused member requests passed7/7; both the new detail keyboard journey and existing compact member journey passed. Final detail screenshots cover1440/390/320, dark mode, empty timeline and last-owner rejection under docs/screenshots/member-details/. Their shared frame/panels/type and compact actions were compared with retained shared-layout/private Rails references. Five locale trees remain in sync. Final acceptance/publication remains controller-owned after independent review and full CI/docs/security gates.
