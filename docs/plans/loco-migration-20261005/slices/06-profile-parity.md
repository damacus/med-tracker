# Profile page parity subplan

Recorded 7 October 2026. Parent: [browser delivery](06-browser.md).

## Outcome and audit limits

Port the entire Rails profile layout and its reachable items to Loco, preserving
tabs, sheets, dialogs, expandable groups, ordering and behaviour in the current
themes. The owner explicitly rejected the simplified draft as the target.
Appearance belongs within Profile settings, not only in the global header.
Do not count a link, placeholder or saved preference with no effect as parity.

This plan records the Rails reference, implementation and verification evidence.
Historical checkpoints below are not the current implementation status.
Reference: `rails/app/views/profiles/` in the retained Rails application.
Implementation checkout: `/private/tmp/medtracker-profile-parity`, branch
`codex/profile-parity-delivery`, based on current main `ce0275584` on 9 October.
The historical `loco-profile` checkout is stale. Preserved avatar work is read-only
input from `avatar-frozen-recovery` and `local-avatar-delivery`; neither checkout
belongs to this delivery. The local four-tab implementation has passed its final automated gate; independent acceptance remains pending.
Security services may exist elsewhere; absence below means absent from this
profile composition, not necessarily absent from the whole application.

## Approved delivery and evidence rules, 9 October

All four tabs and their connected actions remain required in one complete delivery.
Personal API keys move to Security. Closure stays in Advanced's Danger Zone,
using the current Loco confirmation, fresh authentication and sole-owner rules.
Rails presentation is the reference; the current `identity-decision.md` governs
authentication semantics. Its old auth-only staffing and pause statements do not
replace the owner's explicit authorisation of this profile migration.

Every P item below currently has status **unaccepted**. Source evidence establishes
the intended behaviour only. Each requires a failing observable test before new
production code, persisted-result and negative-access checks where applicable, and
rendered desktop/mobile comparison for visible behaviour. Existing API tests and
old screenshots do not establish completion of the new page.

Reference conditions: revision `ce0275584`, isolated Rails project
`mt-profile-reference-20261009`, synthetic fixture accounts only, English (UK),
Europe/London, Command Centre light and dark; desktop 1440×1000 and mobile 390×844,
plus 320px width with doubled text. Record the actual role and settings with each
capture. Ordinary, empty, invalid, denied and relevant error states are required;
keyboard focus, cancellation and correct-tab return are checked alongside images.

Current reuse evidence: main contains profile and notification models/APIs,
push-subscription domain operations, Better Auth security actions and browser
tests, and the shared appearance engine. Main does not register a profile browser
controller. The frozen draft has inline settings and three links, so its design
is not a reference. The later avatar checkout contains additional access/storage
and localisation work to inspect before selecting reusable code.

Do not enable external Gravatar disclosure on the strength of the old checklist.
Reconcile the historical restriction with its original evidence and the current
opt-in semantics before implementing P07.

One leader owns integration and implementation through fixes. Retain the active
leader model for discovery and consequential auth integration; no model escalation
is selected. Codex reported 89% weekly allowance remaining. The explicitly selected
`gpt-6-luna`/`low` agent manages Devin only. It verified CLI 3000.11.3, authenticated
access and `swe-2-max` availability. Plan review precedes
expensive implementation; integrated source review and focused corrections precede
acceptance. Review snapshots exclude credentials and live data; unexpected reviewer
edits are never integrated automatically. No merge, deployment or cutover is allowed.

Historical review jobs: Luna `devin_review_manager` owns review process handle `55046`, snapshot
`/private/tmp/medtracker-profile-review-plan` at `e2629534c7a13ac4af6dc46e23d56256fdb41c24`.
Prompt `/private/tmp/medtracker-profile-plan-review.md`; output and session export
`/private/tmp/medtracker-profile-plan-review-output.log` and
`/private/tmp/medtracker-profile-plan-review-session.json`. Session ID `estimated-roof`.
The exact invocation uses `swe-2-max`, `--permission-mode dangerous` and the
one-run `--respect-workspace-trust false`; no global trust setting changed.
Initial trust rejection created no session. Rails reference startup `40173` and
synthetic seed `60420` completed successfully. The cached test image was used after
Docker Hub returned 504 for the development image. Reference URL is
`http://localhost:62511`; only the new project and synthetic database are used.

Reference evidence now includes the four ordinary desktop/mobile tabs, expanded
notification controls (including automatic Child Patient coverage), avatar and
shortcut sheets, appearance (light/dark), timezone, email-change and closure
dialogs. Files: `docs/screenshots/profile-reference/rails-*.jpg`. Fixture:
`jane.doe@example.com`, ordinary Member, date of birth 1980-01-02, adult with
capacity. Saved timezone London; desktop initial capture uses System resolving
light, mobile uses explicit Light. Dark captures have `dark` in their filenames.
Desktop Profile was visually inspected: sidebar, hero, wrapping tab bar, section
summary and two-column card/settings layout are required. Captures are reference
evidence only; no Loco comparison verdict exists yet. Dialog cancellation returned
focus to the opener; timezone save/reload retained London. Source and rendered
closure copy incorrectly promise deletion of shared records: replace with the
already-approved Loco retention explanation. Ordinary capture is not evidence of
all required invalid, denied, populated and error states; those remain outstanding.
The first focused Loco baseline completed under leader process `72418`, command
`task frontend:test-browser GROUP=profile PROJECT=desktop GREP='profile retains the four Rails panels'`,
log `/private/tmp/medtracker-profile-red.log`. Build passed; the browser test failed
waiting for the absent My Profile link after successful sign-in and household
selection. This is the genuine red baseline, not acceptance. Earlier dashboard
and wizard captures concern deferred work outside this profile migration.

## Item-by-item inventory

Owner scope correction, 10 October: stop general dashboard and medication-flow
work. The complete candidate was preserved at
`profile-scope-checkpoint/20261010T000955Z/` (156 files: 61 changed tracked and 95
untracked; manifest and all SHA-256 checksums verified). The archive and binary
patch are based on `ce02755847cac913a794f3fc0bfc89e5d4201f7b` on
`codex/profile-parity-delivery`. Expanded dashboard, dose-history/timing/navigation,
inventory-wizard and unfinished medication-launcher work is parked intact there.
The active dose/navigation/form files and related journey tests were restored to
the base revision. The subsequent owner correction removes P23–P25 entirely
from profile delivery and acceptance: experiments, wizard variants, dashboard
layouts and launcher variants are explicitly out of scope/deferred. Their
controls and all dashboard-related implementation are excluded from this
candidate. No experiment destination needs implementation or verification here.
Advanced retains exports/privacy, system information and account closure;
API keys remain in Security. This is a Rails-reference migration, not a redesign.
The narrowed renderer is also preserved in
`profile-scope-checkpoint/20261010-experiments-deferred/candidate-before-removal.tar.gz`.
Continue four-tab
profile work and the authorised Devin/Luna review process; do not revive the
parked expansion or count its old checks as acceptance of the narrowed candidate.

Narrowed verification, 10 October: the avatar stream regression first failed
because the whole oversized response was consumed, then passed after applying a
bounded Tokio reader. Fifteen avatar integration checks passed (one fixture-only
helper intentionally ignored). Five reminder checks passed after reusing the
existing identity time-zone resolver for retained Rails aliases. Strict lint,
formatting and documentation build passed. The saved checkpoint was extracted
separately and all 156 checksums passed again. The profile browser sweep exposed
the removed My Profile entry link; only that link was restored in the medication
index, without restoring medication-flow changes. The 66-case browser sweep had
62 passes and four failures: three used the missing entry link, and one checked
the desktop sidebar before setting a desktop viewport in the mobile project.
After restoring the link and correcting that test setup, all 18 cases in the
affected profile test file passed on desktop and mobile. The complete care API
suite passed 241 tests, with one fixture-only helper ignored. Current profile
screenshots are saved under `docs/screenshots/profile-current/`; these are not
yet matched-reference acceptance evidence. Logs are under
`/private/tmp/medtracker-profile-` with suffixes
`avatar-bound-green.log`, `avatar-scoped-domain.log`, `reminder-zone-green.log`,
`scoped-lint.log`, `scoped-docs.log`, `checkpoint-reverified.log`,
`scoped-browser.log`, `scoped-browser-recheck.log`, and `scoped-care-api.log`.

Remaining acceptance gates at that checkpoint were P07 external Gravatar
permission, P28 localized security dialogs and runtime messages, rendered
accessibility review, and matched desktop/mobile reference comparison.
No P-item is promoted to final acceptance by these focused checks. The requested
Devin review was blocked twice by automatic approval review, including after the
owner reiterated prior authorisation; no snapshot was transmitted. The exact
rejection was missing trusted authorisation to export that frozen source payload
to Devin. Local inspection did not establish a completed candidate review;
read-only Devin authentication/model checks also failed because its rolling log
could not be created in the restricted environment. Full `task ci`, final review fixes,
commit/push and PR publication remain outstanding; nothing is merged or deployed.

### Current verification, 10 October

The security forms now use request-scoped language selection and the same five
translation trees as Profile. Known presentation labels and errors are translated;
operation identifiers, credential values and authentication logic are unchanged.
The 12 browser checks pass across English, Spanish, Portuguese, Irish and Welsh,
desktop/mobile, including password confirmation, email, TOTP, keys, recovery-code
acknowledgement errors and the Spanish password journey without JavaScript.
All five locale trees have matching keys. Evidence:
`/private/tmp/medtracker-profile-locales-complete.log` and
`/private/tmp/medtracker-profile-locale-final.log`.

The 20 security browser journeys also passed after localization, including
wrong-proof/replay rejection, passkey registration/removal, TOTP enable/disable,
one-time credentials, cross-session revocation, provider return, verified email
activation and closure preserving shared care. Evidence:
`/private/tmp/medtracker-profile-security-localized.log`.

Rendered comparison exposed the split Security cards and missing current-state
summaries. New browser tests failed on those omissions before their repair.
Account Security and the grouped authentication methods now occupy full-width
cards. Security shows password, TOTP and passkey summaries; Notifications shows
saved reminder/category state and updates after saves; Advanced retains the
export summary and inset disclosures. All 22 profile interaction/layout checks
passed, including 320px doubled text, keyboard tab navigation, focus return,
ordered shortcuts, appearance palettes, saved settings and nullable DOB.
Evidence: `/private/tmp/medtracker-profile-layout-complete.log`.

The first full gate passed 418 Rust tests but found three browser failures among
544 cases: two unused API-key render failures and one push-fixture key panic.
Unused keys now render the localized Never value for null last-used dates. Web
Push rejects malformed private-key lengths before library parsing; the fixture
uses standard JWK export for fixed-width keys. Regression tests reproduced both
failures before the corrections passed. Rendered checks also exposed insufficient
dark-theme closure-button contrast and a stretched notification Save button;
both were corrected after failing browser assertions. The ten focused correction
checks passed on desktop and mobile. Evidence:
`/private/tmp/medtracker-profile-first-ci.log`,
`/private/tmp/medtracker-profile-push-key-red.log`,
`/private/tmp/medtracker-profile-push-key-green.log`,
`/private/tmp/medtracker-profile-visual-a11y-red.log`, and
`/private/tmp/medtracker-profile-corrections-green.log`.

A subsequent full run stopped at an intermittent application-startup
`Sqlx(PoolTimedOut)` in the care API fixture (240 passed, one failed before HTTP
assertions). This matches the open infrastructure investigation #2498; it does
not establish an application regression or a passing gate. The unchanged full rerun subsequently passed. Evidence: `/private/tmp/medtracker-profile-second-ci-pool-timeout.log`.

Final `task ci` passed: 419 Rust tests, two intentionally ignored fixture-only
helpers, all 546 desktop/mobile browser cases, formatting, strict lint and the
Node orchestration checks. Browser runtime was 22.3 minutes. Documentation build
and diff whitespace checks also passed. Evidence:
`/private/tmp/medtracker-profile-final-ci.log` and
`/private/tmp/medtracker-profile-final-docs.log`.

Sixty final light/dark desktop/mobile captures are retained in
`docs/screenshots/profile-current/`, alongside capture conditions and comparison
limits. Visible tabs, grouped security cards, mobile layout, settled avatar and
security dialogs, notification disclosures and Advanced were inspected. Full-page
images retain fixed mobile navigation at its viewport position; dialog images use
the viewport and disable animation. The comparison is not pixel-identical: the
broader Rails sidebar search is absent from the root Loco navigation and native
checkbox switches replace segmented notification controls. The agreed Loco
security and shared-history semantics are preserved. Whole-app accessibility
conformance and external push delivery are not claimed.

A lint error in
the locale render helper was corrected by boxing its framework error. The
synthetic screenshot fixture now stores its time zone in account preferences,
matching the actual schema. No final acceptance is claimed from failed setup
runs. The independent review remains blocked as described above. P07 remains
disabled; no external Gravatar request or email-hash disclosure is introduced.

Avatar integration checkpoint: the invalid-image browser check passed, with
in-sheet error focus and cancellation returning to its trigger. Owned-storage
fixture checks passed (8 database ownership checks and 11 runner checks). The
first full avatar browser run passed upload/replacement/removal/CSRF and invalid
replacement, and exposed stale visible content after membership withdrawal.
The shared form handler now reloads on an authentication/access denial; that
browser correction is under verification. The documented avatar API was absent
and produced a genuine 404 red test; the preserved implementation is now wired
to the current model. Fifteen focused API checks passed, including audit failure
rollback, concurrent changes, durable retirement, active-object preservation,
legacy content rejection and storage failure. The bucket preparation helper is
intentionally ignored outside explicit fixture setup. These tests use complete
synthetic DOB records, preserving the main API's incomplete-profile rejection.
Logs: `/private/tmp/medtracker-profile-avatar-invalid-green.log`,
`/private/tmp/medtracker-avatar-fixtures-green.log`,
`/private/tmp/medtracker-avatar-runner-green.log`,
`/private/tmp/medtracker-profile-avatar-storage-red.log`, and
`/private/tmp/medtracker-profile-avatar-api-fixture-green.log`.
The correction passed all three avatar browser tests in
`/private/tmp/medtracker-profile-avatar-storage-green.log`. The sheet screenshot
was visually inspected with animations disabled; final matched reference captures
remain outstanding. P06 is not yet accepted: other person displays, matching
visual evidence and final integrated review remain. Notifications, Security
composition and exports were still outstanding at that historical checkpoint.

Notification checkpoint: reminder categories and delivery times persist in the
existing model. Managed adult opt-ins use only the current membership's active
manage grants, exclude self, and save atomically with personal preferences;
children and dependent adults are presented as automatic coverage. Three browser
checks passed for grant filtering, invalid-entry retention and forged-selection
rollback; the ordinary save/reload test then passed after explicitly awaiting its
save response before reloading. Logs:
`/private/tmp/medtracker-profile-notifications-managed-green.log` and
`/private/tmp/medtracker-profile-notifications-save-green.log`. The existing four
notification API tests also passed in
`/private/tmp/medtracker-profile-notification-api.log`; the API input shape was not
broadened. P16 browser push/delivery, notification audit-failure and access-race
checks, final layouts/locales and integrated review still prevent acceptance.

### Plan review reconciliation

Security integration checkpoint: the profile now reads authoritative account
methods, passkeys, recovery-code availability and sessions, and uses the existing
Loco security forms in a native dialog. Desktop checks passed cancellation/focus,
password wrong-proof and replay rejection, recovery acknowledgement and result
clearing, authenticator setup/disable, passkey registration/removal, personal-key
issuance/revocation, and cross-session revocation. A delayed acknowledgement
exposed premature dismissal; the corrected check now passes for both Escape and
Close. Advanced closure also passed in place, including sole-owner rejection,
ownership transfer, retained care and revocation of only the closing account's
browser/native delivery endpoints. Logs:
`/private/tmp/medtracker-profile-security-dialog-green.log`,
`/private/tmp/medtracker-profile-security-methods.log`,
`/private/tmp/medtracker-profile-security-pending-green.log`, and
`/private/tmp/medtracker-profile-advanced-closure-green.log`.
Provider linking/unlinking and email verification now return to the initiating
profile in the same browser tab; activation remains gated by the existing email
proof. Logs: `/private/tmp/medtracker-profile-provider-return-green.log` and
`/private/tmp/medtracker-profile-email-return-green.log`. Full locale coverage,
matched desktop/mobile screenshots, additional failure cases and integrated
review remain outstanding. This checkpoint does not accept any P item.

Browser-push checkpoint, 10 October: `web-push` 0.11 provides VAPID signing,
payload encryption and HTTPS delivery. `MEDTRACKER_WEB_PUSH_PRIVATE_KEY` takes
the library's raw base64url private key and `MEDTRACKER_WEB_PUSH_SUBJECT` supplies
the contact URI; the matching public key is derived by the library. Missing
configuration is shown explicitly, and malformed partial configuration fails
startup. No deployment credentials have been read or configured. The transport
compiles, and browser checks pass unconfigured status, CSRF/key validation,
account-scoped subscription registration/removal and Enable/Disable controls.
The control check uses a synthetic browser push adapter against real application
routes; it does not prove external delivery. Logs:
`/private/tmp/medtracker-profile-push-check.log`,
`/private/tmp/medtracker-profile-push-unconfigured-green.log`,
`/private/tmp/medtracker-profile-push-subscription-green.log`, and
`/private/tmp/medtracker-profile-push-controls-green.log`. Delivery outcome tests,
worker registration, scheduled reminder effects, failure cases and final UI
comparison remain required. P16 is unaccepted.

Reminder implementation checkpoint, 10 October: due and missed reminder planning
now uses saved account time zones (UTC by default), reminder times, private text,
the existing dose occurrence projection, active manage grants and adult opt-in.
Missed reminders use a 30-minute grace period. Loco has a registered reminder
task and worker; development/production scheduler configuration queues a run each
minute. The scheduler and worker must run alongside the server; nothing has been
deployed. Recipient discovery uses household-scoped database access, and each
delivery rechecks current access and preferences. Notification intents are audited
and committed before external transport; expired endpoints are removed and
provider acceptance, failure and no-subscription outcomes are retained.

Low-stock requests are recorded atomically with a committed dose crossing the
reorder threshold. Delivery rechecks the preference, active assignment and stock
level; requests older than 24 hours are ignored. A synthetic audit failure proves
that rollback leaves no deliverable request. Duplicate runs do not resend an
already reserved intent. Transport tests use an injected fake, not external push.
Evidence: `/private/tmp/medtracker-profile-reminders-zone-green.log`,
`/private/tmp/medtracker-profile-reminder-worker-green.log`,
`/private/tmp/medtracker-profile-reminder-scheduler-green.log`,
`/private/tmp/medtracker-profile-stock-reminder-rollback-green.log` and
`/private/tmp/medtracker-profile-reminder-lint.log`. The full suite, scheduler
deployment, integrated review, remaining negative cases and final UI comparison
are still outstanding; P16 remains unaccepted.

Export implementation checkpoint, 10 October: Advanced now offers Health data
JSON and Unencrypted ZIP downloads with the retained privacy warning. ZIP uses
the maintained `zip` 9.0.0 writer. The shared portable serializers include health
events, retained doses, outcomes and pause periods; the export-specific scope
excludes view/record-only grants and expired grants. Household managers receive
household scope. Identity secrets and device credentials are excluded. Download
auditing must succeed before the response is returned; CSRF and no-store apply.
Desktop/mobile downloads, ZIP readability, owner/managed/view/expired-grant scope,
audit failure and unchanged native snapshot behaviour passed focused tests:
`/private/tmp/medtracker-profile-export-ui-green.log`,
`/private/tmp/medtracker-profile-export-scope-green.log`, and
`/private/tmp/medtracker-profile-export-native-regression.log`. Matched visual
comparison and full integrated verification remain; P22 is still unaccepted.

Devin `swe-2-max`, session `estimated-roof`, completed its initial read-only plan
review against snapshot `e2629534`; snapshot status and diff remained clean.
Report: `/private/tmp/medtracker-profile-plan-review-output.log`. These findings
are incorporated as implementation requirements, not accepted delivery evidence:

- P16 includes a maintained Web Push/VAPID dependency, configured keys, service
  worker, household-scoped browser subscription/status/test routes, delivery
  registration, expired-subscription cleanup and truthful failure feedback/audit.
  Main has no delivery worker. Do not reproduce Rails' root-path push URL bug.
  Native token delivery is outside this browser surface; closed-account device
  revocation covers browser subscriptions and retained native tokens.
- P19 requires the existing schema's grant preference field in the entity and a
  browser-only write path. Active manage grants, excluding self, determine rows;
  only adults can opt in, dependants remain automatic. Save preferences and grants
  atomically; test deselect-all, view-only denial and cross-membership denial.
  Do not broaden the pinned notification API request contract.
- P22 includes a domain exporter with owners/admins' household scope and other
  members' manage-grant scope, all retained record types, an established ZIP
  library, mode allowlist, download audit and no-store. View grants do not confer
  export permission. No encrypted bundle is added to this browser journey.
- Use native forms/dialogs with progressive, shared same-origin form handling;
  replace only the submitted settings region, retain other unsaved form values,
  keep tab/hash state, announce success/errors and focus the relevant error or
  return trigger. Server responses remain usable without JavaScript. Do not
  introduce Turbo or a new component framework for this port.
- P21 is the existing Loco personal-key service in Security, including explicit
  household/permission/expiry choices and one-time secret display. Legacy native
  app tokens do not become newly issuable profile tokens.
- P04 handles nullable DOB in browser presentation, retaining the existing API's
  deliberate rejection of incomplete records. Household-linked person selection
  stays authoritative; do not copy Rails' first-person association ambiguity.
- P06 must preserve verified image bytes/type/size, tenant/person access on each
  serve, no-store/nosniff, audit, concurrency ordering and durable cleanup/retry.
  Reuse the preserved avatar work selectively and verify it in this checkout.
- P27 revokes device endpoints as part of closure while preserving shared health
  history. Retained export records do not reintroduce Rails' destructive closure.
- New browser writes require CSRF, current membership and applicable grants.
  Ordinary preference saves do not acquire fresh-auth requirements. Direct email
  writes remain rejected in favour of the verified security flow.

The latest 10 October correction excludes P23–P25 and all experiment destinations
from this migration. They are deferred work, not incomplete profile deliverables.
Gravatar disclosure remains a deferred decision tracked in #2502;
any enabled implementation must honour the depicted person's own opt-in and
avoid leaking another person's email hash.

Local implementation evidence, 9 October: `tests/browser/profile.spec.mjs` has four
passing focused desktop tests (`/private/tmp/medtracker-profile-shortcuts-green.log`):
reachable profile/tabs and revoked session, 320px doubled-text reflow, retained
Europe/Belfast save/cancel, and ordered shortcuts including all-empty validation,
preserved entries, reload and actual mobile Inventory navigation. Each new journey
was first observed failing. Nullable DOB now displays Not set in browser; API
validation is unchanged. This is not P-item acceptance: sidebar/visual comparison,
async preservation, translations, remaining settings and all other tabs still need
implementation and broader verification. Do not mistake the initial Security link
or empty Notifications panel for delivered parity.

| ID | Rails item and interaction | Current implementation; independent acceptance pending |
| --- | --- | --- |
| P01 | Hero: avatar, profile title, email and description | Composed identity header implemented and captured at both viewports. |
| P02 | Four tabs in order: Profile, Security, Notifications, Advanced; active section and associated panels | Four accessible tabs with associated panels, active state and keyboard navigation implemented. |
| P03 | Section titles and current-state summaries | Profile, Security, Notifications and Advanced summaries implemented; saved preferences refresh the summary. |
| P04 | Personal information card: name, email, time zone, date of birth, conditional age, person type, capacity and unset values | Read-only personal rows implemented, including nullable DOB and conditional age; API validation unchanged. |
| P05 | Desktop two-column Profile layout, stacked responsive layout | Two-column desktop and stacked mobile layout implemented; narrow doubled-text reflow tested. |
| P06 | Avatar sheet: current identity, supported formats, upload, conditional removal, save/errors and close | Avatar sheet, bounded image handling, access checks, audit, replacement and durable cleanup implemented and tested. |
| P07 | Gravatar preference and explanation within avatar sheet | Disabled pending the external disclosure decision; tracked in #2502. |
| P08 | Time-zone dialog with selection, save, cancel/close and error feedback | Time-zone dialog saves and reloads retained Rails aliases; cancel and focus return tested. |
| P09 | Mobile navigation shortcuts: three ordered slots, None choice and permitted destinations | Three ordered slots with None and permitted destinations implemented; persistence and actual navigation tested. |
| P10 | Appearance sheet: Light, Dark, System; selected states and immediate application | Profile appearance sheet reuses the shared theme engine; immediate application and persistence tested. |
| P11 | Ten palettes: Command Centre, Serene Sage, Modern Clinical, Warm Earth, Deep Lavender, Forest Care, Sunset Support, Tech Indigo, Soft Rose, Minty Fresh | All ten existing palettes retained and exercised with appearance settings. |
| P12 | Security account card: change email and change password with modal navigation | In-place email and password dialogs use existing verified security routes. |
| P13 | TOTP status and enable/disable actions | Authoritative TOTP status and enable/disable journeys composed in Security. |
| P14 | Recovery-code status/count and available management action | Recovery-code status and one-time result handling implemented; acknowledgement and error journeys tested. |
| P15 | Passkey list, empty state, added date, add and remove actions | Passkey list, empty state and add/remove journeys composed in Security. |
| P16 | Browser push status, enable/disable and send-test action | Browser subscription/status/test, service worker and scheduled delivery implemented with a maintained library. Transport tests are synthetic; deployed delivery is unverified. |
| P17 | Reminder master switch and dose-due, missed-dose, low-stock and private-text preferences, descriptions and save | Reminder and privacy controls persist atomically; save, errors and summary refresh tested. |
| P18 | Expandable delivery-time section: morning, afternoon, evening, night | Expandable delivery times implemented with persisted values. |
| P19 | Expandable managed-people section: adult opt-in controls and automatic dependent coverage indicator | Conditional adult opt-ins and automatic dependent coverage implemented with active manage-grant checks. |
| P20 | Advanced accordion: data backup/privacy and system information | Advanced export/privacy and system disclosures implemented; keys are in Security. |
| P21 | API token creation/name, one-time secret, empty/list states, last-used and expiry information, revoke | Scoped personal key issuance, one-time display, unused/used metadata and revocation use the established security service. |
| P22 | Data backup: Health data JSON and unencrypted ZIP downloads, privacy warning | JSON and unencrypted ZIP exports implemented with scoped access, audit and privacy warning; no byte-for-byte Rails format promise. |
| P26 | System information: development worktree/commit, otherwise version/release notes; documentation link | Conditional Loco build/version information and documentation/release links implemented. |
| P27 | Separate Danger Zone: close-account confirmation and cancellation | Closure dialog uses operation-bound proof, sole-owner protection and shared-history retention; endpoint revocation tested. |
| P28 | Localized copy and accessible names, roles, states, panel relationships, errors and announcements | Five profile/security locale trees, semantic controls, keyboard/focus/reflow checks implemented. Destructive-button contrast measured; no whole-app accessibility certification claimed. |
| P29 | Provider linking and unlinking in Security | Existing provider linking/unlinking integrated; configured and unavailable states exercised by security tests. |
| P30 | Active sessions and individual revocation in Security | Authoritative active sessions and revocation composed in Security; cross-session effects tested. |
| P31 | Fresh authentication for each sensitive operation | Existing operation-bound fresh authentication reused; wrong proof, replay and cancellation exercised. |

References: `show.rb`, `profile_settings.rb`, `personal_info_content.rb`,
`security_section_content.rb`, `account_security_card.rb`, `two_factor_card.rb`,
`notifications_card.rb`, `notification_delivery_settings.rb`,
`managed_notification_people.rb`, `advanced_section_content.rb`,
`api_tokens_card.rb`, `data_exports_card.rb`,
`version_info.rb`, `danger_zone_card.rb` and `close_account_dialog.rb`.
Follow their rendered descendants rather than treating unused files as UI items.

## Implementation sequence and ownership

1. Capture rendered Rails reference states for all four tabs and every sheet,
   dialog and accordion above, desktop and mobile. Reconcile conditional states
   against source and tests; expand this inventory if rendering exposes omissions.
2. Write genuine failing browser parity tests for P01–P11, then port shell and
   Profile settings in the existing profile worktree. Preserve existing changes;
   one implementation writer owns profile templates, controller and focused tests.
3. Complete notification presentation and missing push operations, P16–P19.
   Preserve consent, household/person authorization and delivery failure feedback.
4. Integrate Security and API-token presentation against accepted auth interfaces.
   Do not reimplement authentication or edit the independent auth worktree.
5. Complete Advanced, exports and close-account integration in the established
   auth-first lifecycle sequence. P23–P25 are excluded from this sequence and
   its acceptance checks by explicit owner decision.
6. Find and fix keyboard, semantic, localization and responsive defects across all
   states, then independently review the coherent candidate. Run required checks,
   measured full CI and publish reviewed PRs. No merge or deployment.

## Acceptance checks

- [ ] Every in-scope item (P01–P22 and P26–P31) has working implementation and evidence, or an explicit
  unresolved dependency. Dependencies are not passing parity results.
- [ ] Actual tabs expose name/role/selection and panel relationships, support
  keyboard navigation and survive save/validation redirects in the correct panel.
  Port the Rails 320px/enlarged-text regression from
  `rails/spec/system/profile_tabs_spec.rb`; inspect current tab behaviour before
  choosing maintained Loco-compatible components.
- [ ] Sheets/dialogs/disclosures work with keyboard alone; opening/closing,
  Escape where applicable, focus containment/return, expanded state, error focus,
  hidden-panel focus exclusion and unsaved-input preservation are verified.
- [ ] Keyboard shortcuts are inventoried separately from mobile shortcuts; verify
  discoverability, conflicts and editable-field handling for supported shortcuts.
- [ ] Save/reload, invalid input, denied role, revoked access, empty and populated
  states are exercised with synthetic records. Test real operation effects.
- [ ] Theme selection works from Profile across all ten palettes and three modes;
  desktop/mobile screenshots show each relevant layout state and representative
  light/dark themes. Check contrast, zoom/reflow and target sizes.
- [ ] Selected language reaches visible copy, accessible names/hints/errors/live
  announcements and the document language. Preserve native semantics; avoid
  unnecessary ARIA or accessible names inconsistent with visible labels.
- [ ] Use `frontend-design`, `ui-professional`, `translate`, `accessibility` and
  `web-quality-audit` for implementation and evidence-led correction. Per the
  owner, a dedicated screen-reader session is not required. Inspect the rendered
  accessibility tree and automate interaction checks; do not claim tested
  screen-reader compatibility or whole-app conformance from Lighthouse scores.

## Current gaps and next action

P23–P25 and their destination work are deferred outside this migration. The
checkpoint preserves earlier dashboard, wizard and launcher work; none of its
tests or unfinished requirements is a profile acceptance gate.

Implementation and the passing complete Loco gate cover the four tabs and their
authorized actions. Final desktop/mobile light/dark captures are retained for
review. Publish this verified candidate as a draft; independent source review
and the Gravatar disclosure decision remain tracked in #2502. Do not promote
passing checks or publication to migration acceptance, merge or deployment.
P23–P25 remain excluded. The broader migration issue #2450 remains open.
