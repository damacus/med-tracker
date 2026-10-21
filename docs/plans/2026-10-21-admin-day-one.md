# Administration expansion

Approved owner decisions, 21 October 2026. Issue #2707 remains open until its accepted scope is delivered. Each complete vertical slice is a separate PR against main. Unmerged dependencies are named with a focused parent comparison; no stack tooling changes or GitHub merges are authorised.

## Shared boundaries

Rust/Loco is primary; Rails is a read-only visual reference. Use canonical membership operations, restricted runtime credentials, RLS, transactional audit, last-owner protection, CSRF and live authority checks. Keep account sign-in separate from household membership. Synthetic disposable databases only. No live data or deployment. One source writer per slice; controller owns final CI/docs, independent security review and publication. Preserve all five locales, query state, keyboard access, errors and focus. SQL performs bounded filtering and sorting. Use Fish, RTK and repository Task commands, with failing behavioural evidence before executable changes.

## Slice 1: finish and integrate the member directory

Rebase PR #2846 onto current main, preserving main changes and the original branch under a backup ref. Integrate PR #2850 with a history-preserving local merge; leave that GitHub PR open. Retain role editing and active/suspended/revoked status, confirmations, search, role/status filters, name/email/role/status sorting, page size 25, deterministic ordering, pagination and language state.

Fix inactive accountless identities with a narrowly authorised household directory projection returning only membership identity, name and email. Use the same projection for list/search/sort/editor; do not broaden direct people or clinical access. Preserve canonical account identity precedence. Prove restricted-runtime denials for ordinary members, foreign household and stale authority. When a submitted action loses authority, return forbidden and do not mutate; directory rerendering is unavailable because it independently rechecks authority.

Compare synthetic screenshots at 1440, 390 and 320 pixels against retained shared-layout and private Rails references. Source owner runs focused request/browser Tasks and supplies exact RED/GREEN evidence. Controller runs final task ci and task docs:build after source freezes, then independent security review and publication.

## Slice 2: member details and history

Published as PR #2852 from verified slice1/focus-fix dependencies. The member-detail source checkout is frozen; invitation work uses its own managed checkout. Use scoped identity and canonical role/lifecycle commands without changing lock order. Member names link to a detail page with identity, role/status, role powers, clear suspend/revoke/restore effects and explicit distinction from global sign-in and guardianship. Compact visible directory actions retain member-specific accessible names. Preserve original list search/role/status/sort/direction/page/language through detail, role editor and successful/rejected actions.

History is a dedicated typed SQL projection scoped to current household and exact numeric metadata.target_membership_id, restricted to household_membership.role_updated, household_access.membership_changed and household_access.membership_created. Include rejected/no-change attempts with explicit outcome; never claim they changed membership. Page25 newest first with deterministic timestamp/id cursor bounded to the selected member. Reuse safe role/status/outcome allowlists and immutable actor references. Batch same-household actor names through narrow directory projection; fall back to original actor IDs without global identity lookup or raw clinical metadata. Reads add no new audit event.

Acceptance uses restricted full-App requests for member/missing/foreign/ordinary/revoked/locked scope, exact timeline filtering, escaped malicious metadata/name, safe actor fallback, cursor boundaries and navigation through actions. Browser keyboard/action/back links and1440/390/320 screenshots include dark, validation error and empty history. Controller owns final taskci/docs and independent review after source freeze. Slice4 remains deferred.

## Slice 3: invitation management and delivery attempts

Implementing on codex/feat-invitation-management from published slice2 revision 183ec2378, with current main integrated by a normal merge. Provide literal case-insensitive email search, role/status filters and stable keyset pages of 50 with one lookahead row. Bind cursors to household and filters, validate posted context before mutation, and preserve filters/cursor/language through create/resend/cancel and validation responses. Show creation/expiry dates and the latest safe mail attempt; legacy invitations have an explicit untracked state.

Migration 15 retains queued/sent-to-provider/failed attempts without raw email or token. Insert each attempt and its dedicated invitation-worker queue payload atomically in the same TenantTransaction as creation or token rotation. Preserve original inviter provenance and acceptance semantics. Runtime may update only outcome columns, with forced household RLS; cancellation nulls the invitation FK while retaining its original ID. Household cleanup cascades and is not blocked.

The worker sends directly through ctx.mailer. Persist send-start before external contact, then reacquire household→invitation→attempt locks and recheck current invitation, digest/expiry, sender account/membership/version and original inviter authority. Successful replay never sends twice. Uncertain-start recovery records a failed/uncertain outcome without automatic resend; safe guidance issues a new link. Provider acceptance is explicitly distinct from recipient delivery.

Acceptance includes restricted App::boot routes, actual App worker registration/perform, transactional rollback, provider failure, successful replay, duplicate-worker race, accepted-provider/terminal-write failure, uncertain recovery, cancellation/expiry/revocation/supersession, both authorities and cross-household rejection. Verify exact migration catalogue/ledger/idempotency, all five locales and responsive keyboard/screenshots. Controller owns final CI/docs, independent review and publication; no live data or deployment.

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

## Slice 3 focused evidence

Real restricted-runtime delivery first failed because the durable attempt table was absent; browser search/filter requests first failed422. The final affected invitation suite passed56/56, including actual App worker registration, provider receipt persistence, queue rollback, sender/original-inviter rechecks, cancellation retention, successful replay, duplicate-worker race and provider-accepted/terminal-write failure followed by uncertain recovery without another send. Runtime cannot rewrite provenance, delete attempts or cross household RLS.

Posted cross-filter context first redirected303 instead of rejecting422. A pure household/filter cursor check now runs before mutation; request tests prove unchanged token, attempt and version state for cross-filter and foreign-household cursors. Final invitation requests passed3/3, exact migration catalogue/ledger/idempotency passed13/13 and the keyboard/responsive browser journey passed. All five locale trees, formatting and lint checks passed.

Seven synthetic screenshots under docs/screenshots/invitation-management cover1440/390/320, dark, empty, failed/uncertain delivery and validation focus. Comparison with retained shared-layout and private Rails references exposed and corrected a desktop attempt/action grid collision. Target-specific accessible names and visible cancellation email remain clear. Unavailable-authority guidance recommends cancelling and creating a new invitation; uncertain/provider failures retain new-link resend guidance. Final CI/docs, independent review and publication remain controller gates.

The cumulative CI browser run exposed an interaction-readiness boundary after resend. A held interaction.js response proved that new HTML can appear before deferred dialog handlers load; after navigation readiness the same keyboard action opens the target dialog and Escape restores its opener. The journey now waits for existing navigation readiness after resend and duplicate-email validation, preserving dialog content, focus assertions and timeouts. The interrupted cumulative run reached1043/1093 integration tests with only that browser failure recorded; complete final acceptance remains controller-owned.

## Invitation hosted-review corrections

A deterministic duplicate-worker regression exposed a healthy send being suppressed between durable start and send. Household/invitation locks now remain held while a short separate restricted transaction commits the start marker; the outer transaction then locks the attempt and rechecks authority before sending. An interrupted committed start remains uncertain and is never automatically resent. This uses two application database connections briefly. Pool acquisition uses the existing five-second bound; failure before send records unavailable without a start marker, so the existing cancel/new-invitation recovery stays visible.

A total provider deadline now bounds the household write delay while preserving live authority ordering. `settings.invitation_mail.send_timeout_seconds` comes from `MEDTRACKER_INVITATION_MAIL_TIMEOUT_SECONDS`, defaults to30 seconds for absent/null values and accepts1–300 seconds. A deadline is uncertain rather than proof of non-delivery; replay does not send again. Member history now shows immutable account/membership references alongside any escaped current actor name. No historical name snapshot or audit schema rewrite is introduced.
