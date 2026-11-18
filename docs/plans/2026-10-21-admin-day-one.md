# Administration expansion

Current planning: [Unified Administration, 18 November 2026](#unified-administration-18-november-2026).

## Unified Administration, 18 November 2026

Status: the owner approved application implementation after reviewing the HTML
prototype. The latest review decisions below supersede earlier proposals.
Coordinated plan: [modern Profile consolidation](loco-migration-20261005/slices/06-profile-parity.md#modern-profile-consolidation-18-november-2026).
Publication, merge, deployment, live-role changes and data import require their
own authorisation.

Latest prototype review decisions:

- Use one row of feature tabs, with horizontal scrolling at narrow widths.
- Put pending invitations and their existing actions in Users. Remove the separate
  Invitations tab and page.
- Show one card for each manageable household in Overview, with Rename household
  and Manage users. The latter opens that household's Users scope. Remove the
  dedicated Household settings presentation and preserve any other working
  household setting within Administration under the same authority.
- Keep Overview service cards informational: NHS dm+d last successful import and
  current status, open support requests and active sessions, and effective
  registration state with its configuration lock. No service-card buttons,
  duplicate Accounts card or Account security shortcut.
- When deployment configuration locks registration, show its effective value in
  a visually muted registration card with a disabled control. Medicine lookup
  remains independently editable under its existing permission checks.

### Agreed outcome and reference

Move system administration into the normal Administration workspace with one tab
bar. Each feature tab appears according to the signed-in account's permissions;
every request also retains server-side permission checks. Do not create an umbrella
System area, nested system-tool group, visible Household/System section labels or
a duplicate workspace. This owner decision supersedes the earlier grouped layout.
A system administrator without a household must still reach permitted tabs.
The owner chose one shared Users page with an explicit
**This household / All accounts** selector and permission-dependent actions.
Administer the same account in the same place; do not introduce competing user
editors or separate platform-user pages. Preserve registration and medicine lookup
settings. The selector concerns the directory and administration actions, not a
second personal Profile.

Completion requires complete removal of the legacy platform UI after transfer:
no duplicate pages, obsolete header/navigation or links back to that section.
The owner accepts breaking compatibility for this pre-beta redesign: remove old
routes without redirects, duplicate transitional UI or speculative compatibility
layers. Layout, navigation and placement may change;
permission changes are not authorised.

Source baseline: `9faf79b35ca42bdf1cf79bd9cb56ec8d6f6bfc49`. Inspected
`platform.rs`, its support/recovery controllers, `nhs_dmd.rs`, platform and upload
templates, shell links and the profile return integration. Source does not prove
runtime behaviour. The preceding task reports the administrator promotion,
dedicated dm+d storage and uploader navigation merged and deployed with a checked
image; do not repeat those operational changes. This checkout is a separate
reference: its platform navigation does not contain the uploader link, so refresh
the implementation base against the delivered revision before finalising edits.

Read-only reference checkout:
`/Users/damacus/.codex/worktrees/bd02/med-tracker`. It contains unrelated and
uncommitted administration UI work, including `administration.js`, shared shell,
templates/locales and `administration_navigation.rs`. Preserve it intact. Its
intended behaviour includes retained shell during navigation, URL/history and
locale handling, focus, read retry, race cancellation, uncertain-write handling,
unsaved-form protection and ordinary no-JavaScript forms. Reuse these decisions
and inspect the final accepted implementation before extending it; uncommitted
tests are not passing or accepted evidence.

Inspected historical screenshot
`docs/screenshots/shared-layout/admin-platform-recovery-390.png` in that checkout:
it shows the older standalone header, Account security link and separate platform
navigation. Compare against the modern workspace during implementation. No current
runtime baseline or functional acceptance was performed for this plan.

### Feature inventory and proposed placement

| Capability | Proposed workspace destination | Behaviour that must survive |
| --- | --- | --- |
| Household overview/settings | Overview and Household settings tabs | Existing manager authority, household name and explicit AI consent; no global setting writes |
| Household Users | Shared Users, household scope | Search/page, membership detail/role/suspend/restore/revoke, bulk review/retry and last-owner rules |
| Care relationships and guardian recovery | Care relationships tab and its recovery controls | Existing permissions, recovery evidence and guardian safeguards; distinct from system owner recovery |
| Invitations and audit | Invitations and Audit tabs | Filtered navigation, delivery/outbox, authoritative role checks, safe detail and audit visibility |
| Global account directory | Shared Users, agreed All accounts scope | Search/page including accounts with no household; distinguish verified/closed state from sign-in enablement |
| Enable/disable account sign-in | System-authorised Users action | Global effect clearly stated, fresh proof and protection for last usable administrator |
| Grant/revoke system administrator | System-authorised Users action | Independent of household role; current authority, fresh proof, replay protection and audit |
| Registration access | Proposed Registration and medicine settings tab | Open or invite-only state, runtime override/locked state and absent/null fallback to stored setting |
| Medicine lookup settings | Same settings tab | Ordered sources, accessible drag/arrow controls, saved order, existing hidden service URL values and fresh confirmation |
| NHS dm+d | NHS dm+d / Medicine data tab | ZIP validation/limit, upload, queued/extract/count/import/complete/failure, history/counts, safe error reference, refresh and continued background work |
| Support operations | Proposed Support access tab | Request, approval state, activation, expiry, read-only authorised household view and ending access |
| Account-holder support consent | Agreed Profile / Security | Approval and ending remain with the account holder; no automatic approval from system role |
| Owner recovery | Proposed Owner recovery tab | Household search, eligibility, explicit confirmation, usable-owner protection, live recheck and audit |

### Proposed route/navigation mapping

Route spelling is an implementation proposal. Shared Users scope is agreed.
Account-level routes under `/administration/**` are independent of household membership;
there is no `/system` route grouping in this proposal.
Household routes can retain their current `/households/{slug}/admin/**` paths
inside the same shell and single tab bar. Administration entry uses a permitted
tab; a system administrator with no membership sees account-level tabs without an
invented household. Route ownership and permission boundaries do not create
visible navigation groups.

| Existing browser routes | Proposed replacement |
| --- | --- |
| GET `/platform`, `/platform/users` | Administration Users, explicit All accounts scope; preserve query and page |
| POST `/platform/users/administrator`, `/platform/users/active` | Shared Users actions under system-authorised routes, with unchanged proof semantics |
| GET/POST `/platform/settings` | `/administration/settings` |
| GET/POST `/admin/nhs-dmd` | `/administration/nhs-dmd`; transfer refresh and post-upload return |
| GET/POST `/platform/support` | `/administration/support` |
| POST `/platform/support/activate`, `/platform/support/end` | Matching Administration support actions |
| GET `/platform/support/{id}` | Administration support detail with explicit temporary access banner and read-only scope |
| GET/POST `/platform/owner-recovery` | `/administration/owner-recovery` |
| `/households/{slug}/admin/**` and invitations | Retain household journeys in the shared workspace, including deep detail/edit/bulk/recovery routes |
| `/account/security/**`, `/account/support/**` | Keep account operations as required; modern Profile/proof framing and correct workspace return |

Shared Users decision: household managers see the household directory; system
administrators can choose This household or All accounts, retaining one page design
and account editor. A no-household administrator sees All accounts without a fake
household option. Reuse account identity and add permitted global controls within
the same record rather than linking to a competing editor.
Show separate labels for Household role, Membership state, Account state,
Sign-in enabled and System administrator. Changing one must never imply changing
another. A system administrator without manager membership gets no household
management controls. All accounts scope must not expose household clinical information.
Preserve scope, household, search, page and language through proof and return.

Switching feature tabs should retain the modern shell and use
normal URLs with direct loading, reload and back/forward support. Profile's local
tabs currently use `replaceState`; administration page navigation needs its own
history entries. Matching speed and focus does not require identical history
implementation. Never cache privileged content across authority loss.

### Removal inventory and completion gate

Transfer useful handlers into the normal administration controller structure,
then remove obsolete browser routing/rendering from `src/controllers/platform.rs`
and its `support.rs`/`recovery.rs` modules. Domain models named `platform` may
remain: they implement system authority rather than a second UI. Integrate the
dm+d controller/template into the workspace without replacing its worker/storage.

Replace or retire all eleven `assets/views/platform/` templates: `navigation.html`,
`users.html`, `user_actions.html`, `settings.html`, `support.html`,
`support_read.html`, `owner_recovery.html`, `user_access.html`, `user_sign_in.html`,
`user_account.html` and `user_identity.html`. Transfer `assets/views/admin/nhs-dmd.html`.
Remove old presentation styles only after checking Profile support/proof flows
for shared `platform-*` classes. Replace links from `shell.rs`, base layout,
profile navigation, platform forms/pagination, support completion, uploader refresh
and security operation completion. Include application controller registration,
documentation, fixtures and tests in the reference audit.

Remove old platform page/action and standalone uploader routes after transferring
their callers. No bookmark redirects, old mutation endpoints or mutation replay
through redirects are needed. No external consumer of those routes was established
by this inspection. API/mobile/MCP and identity callbacks are separate contracts;
preserve protocol behaviour and transfer validated return destinations. Raise a
specific newly discovered consumer conflict rather than adding a speculative layer.

Completion proof: all retained functions work through Administration; obsolete
templates/render calls are gone; an href/action/redirect scan has no application
links to old pages; direct old-URL requests return the normal removed-route response
and cannot render legacy UI or mutate state. Historical documentation may name old routes
but must clearly identify them as history.

### Delivery slices and acceptance

| Slice | User outcome / completion evidence | Dependency and main risk |
| --- | --- | --- |
| A1: one workspace | One tab bar uses the modern shell and shows each feature according to permission; a system admin without membership reaches permitted tabs | Reconcile existing administration work; test role matrix, direct URL, refresh/back and revoked authority; no Household/System grouping |
| A2: shared Users | Membership and global account administration use one clear page/editor with the agreed This household / All accounts selector | Scope agreed; test search/page, proof/return, cross-household denial, global action labels and last-usable-admin safeguards |
| A3: registration and medicine settings | Registration and medicine lookup can be read, changed and confirmed in their Administration tab | A1; preserve locked config, source order, invalid input and audit/transaction failure behaviour |
| A4: medicine data | Upload, leave, return and inspect progress/history in its Administration tab | A1; preserve queued work/storage and denial; do not re-upload live data for acceptance |
| A5: support access | System operator requests/activates/reads/ends support; account holder controls consent in modern Profile | A1 and Profile security return integration; expiry/revocation/session binding and read-only disclosure are essential |
| A6: owner recovery | Eligible recovery completes in its Administration tab with fresh proof and accurate returns | A1; fresh eligibility, concurrent owner restoration and audit rollback |
| A7: retire platform UI | Every retained capability is reachable through Administration and the old section is absent | A2–A6; exhaustive route/template/link proof, with old routes removed and no aliases |

Slices are separate complete journeys suitable for individual PRs against main
through the delivery controller. Temporary overlap during transfer is not final
acceptance. Do not defer A7 or report migration complete while duplicate UI exists.

For each feature record visible control, expected action, observed result,
persisted state, denied/error state, audit, return/focus and evidence. Check every
button and link; report inert or unfinished controls explicitly. Fixing an exposed
defect must preserve agreed semantics; new functionality requires an owner decision.

Acceptance matrix: anonymous, ordinary household member, manager, system admin
with household, system admin without household, system admin who is not that
household's manager, revoked admin, and last usable administrator. Household role
never grants system authority. System role never grants household clinical access;
support continues to require its existing consent and live temporary authority.
Retain fresh proof, CSRF, replay protection, RLS, transactional audit and owner
safeguards. Include tampered scope/IDs, stale forms, lost responses and audit failure.

Capture baseline and replacement at 1440×1000, 390×844 and 320px/doubled text,
light/dark and all five locales under `docs/screenshots/`; record revision, role,
fixture, timezone and visual comparison verdict. Cover ordinary, empty, loading,
invalid, denied and service-error states. Verify keyboard, selected tab semantics,
focus, mobile wrapping, dialog cancellation, unsaved forms, race cancellation,
uncertain-write reconciliation without automatic resubmission, expired session
and JavaScript-disabled full-page fallback. Preserve sensitive-page no-store rules.

No schema migration, role promotion, live-data operation, import, queue redesign or
new registration default is planned. Existing jobs continue across navigation.
Source rollback restores previous routing/presentation without changing records;
the delivery controller must assess issued links/callbacks before cutover.
Implementation requires failing behavioural evidence before executable edits,
focused acceptance and `task ci`; docs require `task docs:build` and
`git diff --check`, through RTK. This planning exercise has not passed application
acceptance or independently verified the previous deployment report.

### Settled owner decisions and remaining implementation detail

The owner explicitly chose Profile / Security for account-holder approval and
ending of support access. Operator requests, activation and temporary views use
the ordinary Administration / Support access tab. Consent remains accessible to
an ordinary account holder without administration permissions; its authority and
required proof are unchanged.

The earlier question about collecting tools under System is superseded and must
not be asked again. Plan registration/medicine settings, Support access and Owner
recovery as ordinary tabs; exact labels/order are proposals, not new authority.
Support-consent placement is settled. Shared Users is
settled by the owner's explicit first-option answer: one page/editor with This
household / All accounts and controls according to permission. Compatibility is
settled: old routes are removable without redirects. No consequential placement
question remains from this owner Q&A. Proposed new URL spelling and exact tab
labels/order remain implementation details; they do not authorise new behaviour
or authority. The owner approved implementation after prototype review.

Planning verification: `git diff --check` passed. `task docs:build` passed with
five existing missing-page warnings in `2026-10-23-typed-handoff-stack.md`.
Application tests were not run: only planning documents changed.

## Historical administration delivery record

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

Acceptance uses restricted full-App requests for member/missing/foreign/ordinary/revoked/locked scope, exact timeline filtering, escaped malicious metadata/name, safe actor fallback, cursor boundaries and navigation through actions. Browser keyboard/action/back links and1440/390/320 screenshots include dark, validation error and empty history. Controller owns final taskci/docs and independent review after source freeze. Slice4 now has its own implementation and focused evidence below.

## Slice 3: invitation management and delivery attempts

Locally implemented on codex/feat-invitation-management from published slice2 revision 183ec2378, with current main integrated by a normal merge. Provide literal case-insensitive email search, role/status filters and stable keyset pages of 50 with one lookahead row. Bind cursors to household and filters, validate posted context before mutation, and preserve filters/cursor/language through create/resend/cancel and validation responses. Show creation/expiry dates and the latest safe mail attempt; legacy invitations have an explicit untracked state.

Migration 15 retains queued/sent-to-provider/failed attempts without raw email or token. Insert each attempt and its dedicated invitation-worker queue payload atomically in the same TenantTransaction as creation or token rotation. Preserve original inviter provenance and acceptance semantics. Runtime may update only outcome columns, with forced household RLS; cancellation nulls the invitation FK while retaining its original ID. Household cleanup cascades and is not blocked.

The worker sends directly through ctx.mailer. Persist send-start before external contact, then reacquire household→invitation→attempt locks and recheck current invitation, digest/expiry, sender account/membership/version and original inviter authority. Successful replay never sends twice. Uncertain-start recovery records a failed/uncertain outcome without automatic resend; safe guidance issues a new link. Provider acceptance is explicitly distinct from recipient delivery.

Acceptance includes restricted App::boot routes, actual App worker registration/perform, transactional rollback, provider failure, successful replay, duplicate-worker race, accepted-provider/terminal-write failure, uncertain recovery, cancellation/expiry/revocation/supersession, both authorities and cross-household rejection. Verify exact migration catalogue/ledger/idempotency, all five locales and responsive keyboard/screenshots. Controller owns final CI/docs, independent review and publication; no live data or deployment.

## Slice 4: bulk suspend and revoke

Allow selection on the current page only, at most 25 members. Apply per-member changes with partial success and clear individual results. Process the actor last and stop if authority is lost. Preserve last-active-owner safeguards, CSRF, live checks, canonical operations and audit for each accepted mutation. Test mixed success/failure, stale authority, actor ordering and mobile/keyboard interaction. Implementation uses a separate bulk checkout, fast-forwarded to the invitation readiness correction. Strictly validate positive distinct IDs, action and confirmation before any mutation; foreign/missing/state-invalid targets produce safe individual failures without blocking eligible local targets. The current-page restriction is the UI selection boundary plus the server limit, not an all-or-nothing page-membership requery. Preview names use only a bounded current-household identity projection. Preserve filter/page/language context, retain failed selections for explicit review/retry, and never reread or render the directory after actor authority is lost. Each target uses a fresh principal and transaction with canonical household→target locking. Commit canonical validation rejection audits; rollback system failures. Mark a commit failure unconfirmed and stop further processing. Failed and unattempted selections remain available for explicit review only after a fresh manager check succeeds; never retry automatically. No bulk restore, new roles or public API changes. Controller owns final cumulative CI/docs, independent review and publication.

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

## Slice 4 focused evidence

The restricted App::boot preview request first failed404 before implementation. Focused bulk requests now pass11/11: named non-mutating preview, strict selection/confirmation/CSRF bounds, mixed eligible/state-invalid/foreign/missing targets, escaped identities, preserved filter/page/language and failed-only retry. Ordinary/revoked/locked actors and inactive households cannot mutate. Canonical rejected audits commit; audit failures rollback. A deferred commit rejection preserves an earlier committed success, marks only the failed commit unconfirmed and leaves remaining targets untouched. Concurrent batches retain an active owner, and actor-last self removal returns standalone results without directory links or a directory reread.

The keyboard journey verifies select-all chooses25, page/filter changes clear selection, named effect confirmation, partial results and failed-only review. Mobile column headers stay in the accessibility tree with explicit row/header associations. Eight synthetic screens under docs/screenshots/bulk-member-access cover1440/390/320 selection, confirmation, results, dark, empty and validation focus. They retain the shared member frame, rounded panels, compact actions and responsive labels compared against retained shared-layout and private Rails member references. Formatting, lint/dependency checks, frontend tests and all five locale trees pass. Controller owns final cumulative integration, CI/docs, independent review and publication.

## Shared-layout review follow-ups

A separate follow-up fixes stale sidebar avatars after asynchronous profile saves, strict explicit language handling for the main People browser routes/forms/pagination, and the overview expired-invitation link. Real avatar upload/removal updates sidebar, header and profile images without replacing the document/account dialog. Browser Tasks now provision disposable RustFS using the existing integration pattern so that journey uses real storage.

People card counts now use two grouped typed SQL projections with the same canonical source/medicine scopes and exact existing per-source semantics: active/non-retired sources, inclusive schedule dates in the configured care timezone, and inactive-as-paused exclusion. Counts deliberately remain per source rather than distinct medicines. Restricted request acceptance covers repeated sources for one medicine, date/inactive/retired exclusions, a non-UTC care date and foreign-household sources. The count baseline passed before optimization, then the same acceptance passed afterward.

The current member table already has visible mobile Email/Role/Status labels; its source and retained mobile capture were checked and left unchanged. Cedar still denies delegate_person, so the care-warning assignment action is not exposed and was not fabricated or changed. The visible People detail carer link uses its existing locale parameter and the real read route renders Spanish. No guardian-recovery, membership-policy or public API change is included.

Focused requests passed3/3, final new browser journeys2/2, existing related People/admin browser checks8/8, frontend19/19, strict lint, formatting and five-locale synchronization. Four synthetic desktop/mobile captures live under docs/screenshots/polish-followups/. Independent review, combined CI and publication remain controller-owned.

## Invitation hosted-review corrections

A deterministic duplicate-worker regression exposed a healthy send being suppressed between durable start and send. Household/invitation locks now remain held while a short separate restricted transaction commits the start marker; the outer transaction then locks the attempt and rechecks authority before sending. An interrupted committed start remains uncertain and is never automatically resent. This uses two application database connections briefly. Pool acquisition uses the existing five-second bound; failure before send records unavailable without a start marker, so the existing cancel/new-invitation recovery stays visible.

A total provider deadline now bounds the household write delay while preserving live authority ordering. `settings.invitation_mail.send_timeout_seconds` comes from `MEDTRACKER_INVITATION_MAIL_TIMEOUT_SECONDS`, defaults to30 seconds for absent/null values and accepts1–300 seconds. A deadline is uncertain rather than proof of non-delivery; replay does not send again. Member history now shows immutable account/membership references alongside any escaped current actor name. No historical name snapshot or audit schema rewrite is introduced.


## Bulk landing review correction

Initial bulk review now accepts only IDs on the exact current filtered directory page. A typed, bounded session review binds the actor, household, native session digest, action, selected IDs and return context to a short-lived UUID. Confirmation consumes that review; results permit only failed/unattempted IDs for explicit retry even when earlier changes shift the directory page. Returning to the member list cancels a pending review. This uses the existing Session consumption semantics and does not claim distributed exactly-once processing.

The filtered-out and unreviewed-confirmation regressions first failed200 instead of422. Focused requests now pass18/18, including off-page and tampered/cross-session rejection, replay, cancellation, shifted-page failed-only retry and the retained canonical concurrency/audit/commit-failure scenarios. The keyboard/responsive journey passes with all eight existing bulk screenshot hashes unchanged. Selected identities and ordinary bulk fixture CRUD use SeaORM/SeaQuery builders. Three isolated synthetic PL/pgSQL trigger fixtures remain solely for authority/audit/deferred-commit fault injection because the installed schema builders expose no function/trigger creation API. Final cumulative CI and publication remain controller-owned.
