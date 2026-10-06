# Migration progress

Updated 6 October 2026. Complete every slice in [the plan](plan.md).
Foundation is accepted on the published commit below. The remaining migration
areas are incomplete. Nothing is merged or deployed.

A fresh read-only CLI process used the newly configured GitGuardian MCP with the
existing OAuth connection. The authorised `get_incident` call verified incident
37907524: Authentication Tuple, TRIGGERED, five occurrences. Occurrence content
was excluded. No incidents have been resolved; the external GitGuardian gate
remains open. This chat has not yet loaded the new tools.

The current local delivery implements invitations, open/invited signup, verification
and resend, stored API credentials, medication reads and dosage management. Focused
checks pass; Devin SWE-2 Max accepted the final account and care sources. Combined
`task ci` 40067 passed all Rust groups and 155 of 156 desktop/mobile cases.
The sole failure was EPERM while polling server cleanup after SIGTERM; the
application assertions passed. A one-line fix conservatively treats that liveness
response as still alive, without accepting it as cleanup or suppressing signal
refusal. The helper passes five regression/real-process tests. Exact failed mobile
case 85015 now passes, including teardown. This is combined scope evidence with
a focused tooling repair; the original full CI exit remains failed, rather than
being described as a fresh successful full run. Six additional screenshots are
preserved. Publication and exact-head hosted verification remain required.

## Approved cutover scope — 6 October

Rollback restores an independent pre-cutover dump or frozen replica and restarts
Rails; new Loco data may be lost. Prove restoration and Rails operation, rather than
Rails compatibility with Loco-created data. Stop Rails servers, workers and schedulers
before Loco starts; stop Loco writers before restoring Rails. Old pending/failed Rails
jobs may be lost without queue transfer or individual delivery reconciliation.

Unsupported historical passkeys may be invalidated without counting affected accounts
as a gate. Invalidate existing sessions/tokens with a proved mechanism, preserving
fresh sign-in and MFA. Historical links, export formats, offline queues and push
subscriptions need not survive. Preserve records/files, new authorised downloads,
new exports, future offline replay and future push behaviour.

Use daisyUI, the same colour schemes and clear Loco browser routes. Exact Rails pixels,
CSS geometry, theme-export fidelity and browser bookmarks are excluded. Required
information/actions and accessibility remain. PDF reports must match existing appearance
and work in scratch. Inspect renderer/font and retained Rust work before choosing.

FHIR/SMART, MCP, AI and external medication lookup are deferred from the first
production gate, with no exact external provider/configuration parity requirement.
They remain in the full migration goal. Core medication entry/care remain mandatory.
Report first production readiness and complete migration acceptance separately.
None of these design decisions authorises live data access, invalidation or deployment.

The subsequent first-release answers keep NHS dm+d import/reconciliation/scanner,
medication reviews/background refresh, complete administration, time-limited support,
household export/closure/retention holds/deletion, avatar uploads, complete devices/
sessions and automatic live dose/stock updates in the core production gate. Offline
capture/replay, portable imports and both-native-app acceptance are deferred from
that gate but remain in the full goal. Optional AI/provider deferral cannot exclude
catalogue/scanner or review outcomes.

## Published

The application delivery at `e920fbc003ee4bbe706a5984cf56a7febe7ba4e6` is pushed to
[draft PR #2451](https://github.com/damacus/med-tracker/pull/2451).
Current published HEAD is signed commit `3864ef9d688482041e2363f0956d8233a8936e60`,
which changes only the Loco CI timeout.
Loco owns root commands, routes, configuration and templates. Rails is independently
runnable under `rails/` as the reference and rollback application.

Published workflows include persistent browser sign-in, permitted household
medication pages, medication create/edit/guarded deletion, dose recording and stock
changes, location management, retained TOTP and recovery sign-in, People management,
household administration, passkey registration/sign-in/removal and native/SMART
OAuth consent, token exchange and revocation. Local full CI passed with 41 care API
and 124 desktop/mobile cases; final Administration and Passkey reviews are clean.
Documentation checks and eight inspected screenshots are published. GitHub checks
for this new commit stopped at the Loco job's 20-minute timeout after browser
case 94 of 124 started. The other selected suites passed; the final gate rejected
the cancelled job. The reviewed, published workflow change allows 60 minutes without
removing checks; `task ci:check` passes all 94 tooling tests. Exact-head
[CI 37427833670](https://github.com/damacus/med-tracker/actions/runs/37427833670) is
successful: all 39 jobs passed, including the full Loco job. PR validation
37427833321 passed. GitGuardian remains the separate outstanding check.
The previous `bda18ac` delivery's
[GitHub CI](https://github.com/damacus/med-tracker/actions/runs/37414918830)
succeeded on that exact previous commit, including Loco, Rails rollback and native clients.
GitGuardian remains a separate unresolved check;
its exact-head check 112151424072 reports 15 detections across 21 PR commits. Inspection confirms
fixed OAuth audit labels (incident 37907524) and synthetic OTP derivation vectors
(incidents 37785602, 37785603, 37785604, 37910787 and 37910788), plus synthetic
passkey fixture and foreign-credential test values (incidents 37913501–37913505).
The fixtures record their Rails FakeClient/FakeAuthenticator provenance and mark
their private keys as test-only. The foreign test IDs decode to fixed fixture
labels; they are not production credentials. These are false positives.
The user approved access and resolution of these verified false positives.
The official hosted GitGuardian MCP is configured and an authorised read-only
CLI call verified incident 37907524: Authentication Tuple, TRIGGERED, five
occurrences, with occurrence content excluded. Incident/source-only scopes are
saved. The MCP's `manage_private_incident` requires an explicitly user-supplied
ignore reason; that choice is pending for verified audit-label and synthetic
fixture detections. No incidents have been resolved yet.
The check is still failing. No scanner exclusions or history rewrite were made.

## Published behaviour and evidence

Published Recovery passes all 14
desktop/mobile cases, including exhausted OTP, passkey-only recovery, one-use
codes, current-account checks and bound OAuth continuation.

People passes eight API cases and its desktop/mobile browser journey. Creation
preserves capacity, carer/grant/Home associations, permissions and audit/sync
effects. Partial updates retain the contract without mandatory `If-Match`.
An actual Rails service probe supplied the exact saved POST/PATCH retry digests,
including filtered email; both replay correctly. New Rust keys still bind the
submitted body. Forced audit failure rolls back every dependent write. A reproduced
lost permission-version increment is corrected with an atomic stored-value
increment. Extra request fields are filtered as Rails does, protected account IDs
cannot be assigned, and an empty filtered person returns 400.

Devin SWE-2 Max approved both Recovery and People corrections. Final combined
`task ci` passed all Rust/static/lifecycle checks, 31 care API cases and all 74
desktop/mobile browser cases. Owned test containers, volumes and networks are
absent after cleanup. The pre-correction partial run is superseded by this result.
The corrected delivery is committed and pushed. Household administration now passes
eleven API cases and six desktop/mobile journeys. Devin SWE-2 Max accepted the final
requirements, correctness and security review with no outstanding defects.
Administration is published with the passkey delivery below.

Native passkey registration, passwordless and second-factor sign-in, restart and
password-confirmed removal pass. Supported ES256 and RSA-2048 RS256 credentials
generated by Rails WebAuthn 3.4.3 sign in through the native browser without
re-enrolment, advance their retained counters and survive a server restart. Import
does not create a registration audit. The maintained library verifies the ceremony;
there is no custom signature verifier. A real concurrent removal reproduced an
account/key/session foreign-key lock-order failure; consistent account-before-key
locking now produces a clean rejection without issuing a session.

The user approved dropping unsupported historical passkey algorithms on 6 October.
This intentional compatibility break is recorded in [the deployment guide](../../deployment.md#approved-cutover-decision-unsupported-passkeys).
Supported keys work on mixed-key accounts. Security settings identify keys needing
replacement, and unsupported-only accounts receive recovery instructions after
password verification. Recovery, supported-key registration and explicit old-key
removal pass without bypassing MFA. Unsupported rows remain for Rails rollback
until authenticated removal. All fourteen focused transition, import, concurrency
and CSRF/ownership cases pass across both viewports. Devin SWE-2 Max accepted the
final requirements and correctness/security review with no outstanding defects.
Final combined `task ci` passed all Rust/static/lifecycle checks, all 41 care API
cases, three unit cases including exact challenge expiry, and all 124 desktop/mobile
browser cases on the frozen candidate. Documentation verification, commit and push
are complete for administration and passkeys. This does not accept
the remaining identity, care, browser or release requirements.

## Current implementation

Invitations and account setup are next. A real signed-in browser reached household
administration and failed at the absent Invitations link. Existing API tests also
proved the missing invitation routes. Nine invitation API tests and four persisted
credential tests pass. Desktop/mobile invitation management passes with real Loco
worker email delivery, resend and cancellation through an isolated Mailpit inbox.
Guest password and OTP sign-in resume the invitation. Acceptance committed the
expected effects, but a stale synthetic fixture ID sequence changed the selected
account person. After correcting the fixture, all six invitation journeys pass
on desktop/mobile, including password/OTP acceptance, atomic effects and replay.
Stored sessions and app tokens now pass real care-route checks, including source
household binding and immediate permission-version withdrawal. Four API-session
and five resource-scope regressions also pass. Account creation with actual email
verification remains required in this delivery. The missing POST handler is now
registered. A valid submission initially returned 503 because its conflict rule
did not match the preserved partial email index. The corrected insert passes the
representative account creation, real email delivery and verification journey.
All ten desktop/mobile signup cases now pass, including invalid, expired, revoked,
cancelled and existing-account rejection. All ten open-registration desktop/mobile
cases also pass: first-household bootstrap and verification, environment overrides
and closed-registration denial. A database permission test proves that owner
detection exposes only a boolean and preserves the application's tenant row
visibility. Final verification resend passed both desktop/mobile cases with real
SMTP delivery, strict recent-email suppression, retained key reuse and verified/
missing-account refusal. Completed queue jobs and verification-key rows remain
stored, matching the pinned framework and Rodauth lifecycles; tests count those
rows while proving that rejected requests add no effects.
Eight dosage API/removal cases pass, covering protected inactive references,
live manager revocation and audit rollback. Both desktop/mobile dosage browser
journeys pass actual create/edit/delete and parent-stock changes. Deleting the
last tracked option resets stock to untracked, matching Rails. Medication
list/detail reads now pass both route tests after two actual missing-GET cases
failed, including conditional reads, stock projections, visibility and auditing.
Valid OAuth credentials do not substitute for the invitation API's
required user session. These changes remain local and have not passed acceptance.

## Whole-migration acceptance

| Area | Implemented evidence | Still required |
| --- | --- | --- |
| Foundation | Accepted: both runners, audit, corrective review, local/docs and exact-head hosted CI pass; record published | Complete; product and populated rollback gates belong to later areas |
| Persistence | Guarded schema adoption, restricted roles/RLS, persistence and tenant tests, normal seed CLI | Populated Rails rollback and complete schema acceptance |
| Identity | Password/session and bearer tests; OAuth library integration; published TOTP, Recovery and supported passkeys | Account creation/verification, remaining security settings and complete client interoperability |
| Care | Dose, stock, medication, location and People management; published household settings, membership and care-access administration | Invitations, remaining household operations, dosage, treatment and sync |
| APIs | Actual Loco listener checks for migrated care and OAuth routes | Complete native/FHIR/SMART/MCP contracts |
| Browser | Tera/daisyUI themes and migrated care/OAuth journeys on desktop/mobile | Remaining workflows, accessibility evidence and PWA/offline |
| Workers | Owner-provisioned queue and restricted queue startup/recovery configuration | Every real job, recurrence, reminder, retry/idempotency and crash/drain outcome |
| Release | Retained source/PR inputs and rollback application preserved | Both scratch architectures, complete journeys, populated rollback and obsolete tooling removal |

## Working rules and reports

Use [the current working rules](team-charter.md). Historical relocation/workspace
and source-preservation audits run explicitly through `task migration:audit`.
Do not regenerate historical ledgers for each delivery. Duplicate runners, unused
demos and the proposed temporary worker probe were removed. Keep real database
isolation, process cleanup and side-effect verification.
The current runner has no duplicate replacement to remove. One representative
browser test now establishes a shared missing entry point; all distinct behaviour
and security cases still run after implementation. Retire helpers when their last
caller is replaced, within the same delivery.
The remaining plans use named model methods and existing tenant transactions.
The proposed command dispatcher, execution wrapper and parallel household test
binary have been removed from the plan. The existing browser command accepts a
standard test filter for prerequisite checks; it remains the full-suite runner.

Migration defects stay in this work. Create follow-up issues only for independent
scope. Useful retained PR inputs are recorded in [pr-inputs.md](pr-inputs.md);
close superseded work only after its replacement is published. Incident #2453
remains separate. Original Rails tests remain the rollback reference; Loco work
does not trigger unrelated RSpec repair.

Update the local reports at `/private/tmp/medtracker-migration-status.html` and
`/private/tmp/medtracker-2450-review-status.md` at each completed slice or material
blocker. Detailed logs stay outside the status page.
Prior delivery history is preserved in Git and the existing evidence artifacts.
Last recorded retrospective: 08:14 UTC on 6 October. Next: 10:14 UTC.
