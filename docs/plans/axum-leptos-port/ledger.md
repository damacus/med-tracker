# Axum and Leptos port ledger

Plan: `docs/plans/axum-leptos-port/plan.md`

Current goal: [goal.md](goal.md). The user authorized the workflow-first
milestones and continued execution on 25 September. The app-level full
cutover-readiness goal now reports active. The previous turn made concrete
progress: reviewed direct dose recording was committed and pushed as `39904af0`.

## First-party session checkpoint

Standalone web login now opens the first active household dashboard. Signed
database-backed browser sessions can call the shared API, with current actor,
membership and person permission checks, CSRF/origin enforcement, sliding
expiry and logout revocation. Focused session acceptance passed 8 HTTP cases;
canonical regression passed 36 HTTP and seven browser cases with no skips.
See [session evidence](journey-web-session-runner-report.md) and
[session review](journey-web-session-review.md).

The dashboard remains a minimal authenticated landing page. The next work is
the shared people/schedule/assignment reads and the full Leptos medication
journey, including the confirmed Escape-focus correction. The corrected
shared-read suite passed all four cases against Rails and failed all four
against Rust's missing routes (404). Both isolated Compose runs cleaned up
successfully with unchanged source manifests. See
[shared-read evidence](journey-web-reads-runner-report.md). This establishes
the failing tests required before implementing those routes.

## Direct dose checkpoint

Direct medication-take POST and history GET now pass independent requirements
and code-quality review. The stable combined run passed 36 HTTP cases and five
existing OAuth browser cases. Dose coverage includes exact stock arithmetic,
schedule and assignment sources, tracked inventory, authorization, request and
clinical audit records, concurrent retries and cross-household UUID conflicts.
See [dose review](journey-dose-review.md) and
[runner evidence](journey-dose-runner-report.md).

Known Rails defects are corrected: contradictory UUID retries return 409 and
contradictory supplied units are rejected. These are deliberate changes to
reconcile with the shared OpenAPI document before cutover. The passing browser
cases cover secure entry; the first-party Rust dose journey remains next.
Its [session and shared API boundary](journey-web-boundary.md) and
[tests](journey-web-api-tests-brief.md) are prepared. The Rails browser baseline
also identified missing Escape focus restoration, which Rust must correct.
The final Rails browser baseline passes both desktop/mobile dose, stock and
history journeys plus the invalid-login/foreign-record denial case. Its two
strict focus cases still fail and remain required corrections. The dedicated
web-session HTTP baseline fails all four cases at the missing standalone
login CSRF form, establishing RED for the next implementation.

## Secure entry checkpoint

The Rust public mobile OAuth path now supports CSRF-protected password login,
consent, S256 PKCE exchange, refresh rotation, revocation, household selection
and medication reads with the issued token. Unsupported enrolled factors fail
closed. The combined run passed 30 HTTP and five actual browser checks with
matching complete before/after runtime manifests. See
[runner evidence](journey-auth-runner-report.md),
[independent review](journey-auth-review.md), and the
[product boundaries](journey-auth-product-report.md).

Screenshots exposed an inherited unstyled Leptos form that the original
presence-only browser assertions missed. Responsive form styles and geometry
checks now cover desktop/mobile, keyboard login errors and usable consent.
This does not establish full Rails visual or authentication parity.

Next: [direct dose and stock/history journey](journey-dose-brief.md), including
the missing first-party web sign-in boundary. MFA/passkey/recovery completion,
password reset and consent cancellation remain open. The full migration,
PWA, API and combined-memory gates remain in [goal.md](goal.md) and
[issue 2299](https://github.com/damacus/med-tracker/issues/2299).

## Current execution override — 25 September 2026

The user's subsequent instruction authorized bounded Rust implementation while
the remaining parity inventory stays open. The medication read implementation
is integrated through `7bec286a`; earlier entries saying no Rust API exists
describe their historical checkpoints. SeaORM 2.0.3 is in use; the planned
comparative ORM benchmark is not claimed complete.

The current run follows [team-charter.md](team-charter.md), superseding the
older ownership table below. The user explicitly authorized separate parallel
test and product writers. Root owns integration; Sol owns each writing lane;
Luna owns objective execution; independent Sol review gates acceptance.
Read [retro-2026-09-25.md](retro-2026-09-25.md) for the workflow evidence.

Assigned: [mobile OAuth medication reads](mobile-oauth-run-2026-09-25.md).
Both interrupted predecessor agents handed off with no edits or processes.
No new worktree is needed for this disjoint-file tranche. Remaining ledger
trigger proof, broader API/UI/PWA parity, and representative memory/load
measurements remain open.

Mobile OAuth red proven against unchanged `rust/api` at `7bec286a`: valid
mobile list request returned 401 instead of 200. Fixture preparation took
122 seconds; the failing test took 0.01 seconds. Product implementation is
now active alongside further test authoring.

User correction: host port 39998 is an avoidable runner design constraint.
[Compose runner correction](compose-runner-brief.md) is assigned to a disjoint
tooling writer. API and HTTP tests must run on each disposable project's
internal network; actual overlapping runs must prove isolation.

Mobile OAuth green: two concurrent isolated Compose runs each passed 19/19
HTTP cases (seven OAuth, nine medication read, three forecast). Unit tests
passed 3/3; Rust formatting/Clippy and fixture RuboCop passed. Final independent
review records are clean in the run reports. No broader API/UI/PWA
parity or current memory/load target is claimed.

Compose runner tranche accepted: the explicit-subnet `task contract:isolation`
gate passed fixture separation on PostgreSQL 18.6, successful cleanup without
affecting the peer, deliberate failed-target cleanup, and final peer cleanup.
No owned contract networks remained. The new API sidecar has no published
host port. The two 19/19 application runs have a matching 43-file digest, but
two new runner paths were omitted; exact API lifetime overlap was not captured.
Those evidence limits remain recorded rather than being promoted to stronger
claims. Requirements and code-quality review found no actionable source issue.

Next delivery milestone: medication viewing, dose recording, and updated
stock/history as one usable journey. Broader inventory work is a cutover
checklist, not the primary measure of progress.

Assigned next: [secure journey entry](journey-auth-brief.md), implementing
real login/OAuth issuance and household selection before direct dose/history.
The existing OAuth corpus supplies the first red; no new broad inventory is
required. The full cutover objective is preserved in [goal.md](goal.md).

Planning baseline: `781c3a297ffe24ae5b70f893d55b568eab1216fc`.

## Decisions

- Observable behaviour parity replaces one-for-one RSpec translation.
- Existing accounts and medical data survive cutover; reauthentication is
  acceptable.
- Combined application-process target is under 200 MB; PostgreSQL and
  supporting services are excluded.
- API tests precede broad UI parity tests; product implementation follows both
  baselines except the bounded Leptos foundation ruling below.
- Persistence will be ORM-backed. Select between stable Diesel Async and
  SeaORM after representative PostgreSQL 18 operations and application-memory
  measurements; reserve parameterized SQL for narrow ORM gaps.

Ruling: ORM-backed persistence is required by the active cutover goal — a
PostgreSQL 18 trial chooses the ORM before Rust API implementation — the cost
if wrong is reworking the persistence boundary, not dropping the ORM
requirement without a new product decision.

Read-only ORM trial design: compare the authorised medication read and atomic
stock adjustment with identical Rails-migrated PostgreSQL 18 fixtures, pool
limits, HTTP boundary, and workload. Audit persistence and lock behaviour are
correctness gates; warm-idle and loaded application RSS, CPU, and latency
choose between candidates only after equivalent results. No benchmark or ORM
selection has been claimed.

Ruling: for v1 `show_hidden`, only `1` reveals hidden medication review prompts;
any other value retains Rails' 200 response with hidden rows excluded. The
OpenAPI enum describes valid client inputs, while the existing controller and
Rust contract establish this safe compatibility behaviour for invalid input.
The cost if wrong is continuing to accept an invalid value; changing it to a
rejection later would need an explicit API compatibility decision.

Ruling: start one isolated Leptos SSR login-page foundation before the full
API and browser baselines close — the user explicitly requested an early UI
start with an expected-red Rust smoke — the cost if wrong is reworking the
shell when shared API, router, and authentication interfaces are settled.
This does not count as UI, PWA, or workflow parity.

Ruling: port the existing login-form display example in
`spec/system/user_sessions_spec.rb` and the public heading/OIDC-absence
example in `spec/features/security/oidc_security_spec.rb` as the foundation
smoke — the Rails suite is the behaviour source, and the user rejected an
invented substitute — the cost if wrong is later reworking this small smoke
when the full authenticated login journey is ported.

Ruling: the portable Chromium smoke excludes OIDC/SSO provider buttons while
allowing `Continue with Passkey` — Rails reveals that separate passkey control
in a real browser although the existing no-OIDC system spec uses a broad
`Continue with` matcher under its test setup — the cost if wrong is missing a
newly named OIDC provider button until the full authentication browser suite.

| Owner | Exclusive paths | Reserved shared paths |
| --- | --- | --- |
| API contract writers | Their separate worktrees; `rust/contract-tests/**`, `rust/parity-matrix.md`, API fixture and contract-task changes | No `rust/web/**` edits |
| Leptos foundation writer | Separate worktree and branch; `rust/web/**` only | No root workspace/lockfile, shared router, database/schema, API fixtures, parity matrix, or contract runner edits |
| Coordinator | Plan, ledger, serial integration and combined checks | Root workspace/router/API integration after UI review |

## Progress

- Tranche 0: API inventory and disposable contract runner accepted through
  reviewed API-plan Tasks 1 and 2; the matrix records all 118 OpenAPI
  operations and the existing RSpec inventory.
- Tranche 1: in progress. Reviewed API contract slices through web JSON reads,
  web JSON actions, cross-account session denial, platform/web profile CSRF,
  web JSON action CSRF, and push delivery are integrated. The full isolated
  Rails contract corpus passed with 228 tests, 0 failures, and 12 ignored
  cases after the FHIR fixture, shared-stock expectation, and sync pause
  precondition were corrected.
  The feature-disabled AI case also passed in its separate target. Known
  Rails privacy, clinical-input, and image-representation defects remain
  explicit gaps.
  The sync change-feed privacy regression is integrated: focused Rails and
  Rails-backed Rust contract checks passed, as did the isolated full Rails
  suite (6,102 examples, 0 failures). The integrated privacy contract passed
  1/1. Legacy ownerless rows and deleted-Person tombstones remain manager-only;
  restricted clients recover by replacing local state from a full snapshot.
- Tranche 2: full browser/PWA parity pending. The bounded Leptos login-page
  foundation is integrated and passed its desktop/mobile Rust browser smoke;
  it does not close this tranche or prove authentication, PWA, or visual parity.
- Tranche 3: pending.
- Tranche 4: pending.
- Tranche 5: pending.

Record each accepted task as `Task N: complete (commits BASE..HEAD, review
clean)` and each unresolved decision as `Ruling: decision — evidence — cost if
wrong`. Do not mark a task complete from a test run without its independent
review and recorded result.

Accepted API slices: platform/web profile CSRF (`aaf11f35`, independent review
clean); push delivery (`fd82ca4f`, independent review clean); web JSON action
CSRF (`8b96cda8`, independent review clean). Push delivery's combined-run
target list was corrected after the fake runner first failed; the full
isolation and failure-aggregation test then passed. The full Rails corpus
first failed because the web scan test expected the fixture's original stock
after earlier dose tests mutated it. The corrected test (`f9a8bf0c`) compares
the web scan with an authorised live API read, allowing only the two endpoints'
documented decimal display difference; focused Rails passed 7/7 and the full
corpus passed 227/0/13. These are contract tests against Rails, not a Rust API
implementation.

Bounded Leptos foundation Task 1: complete (`b14a8351..a86710ab`, independent
review found no actionable issue after checking the report and four
screenshots). Rails and Rust browser smokes each passed 2/2 across desktop and
mobile. Integrated Rust unit, formatting, lint, and browser smoke passed; the
browser smoke passed 2/2 after the pinned Playwright dependencies were
installed with `10fa0129`. The corrected portable browser smoke did not have
a strict failing-test-before-production sequence, which the writer reported.
The visible `Forgot?` link currently leads to an unimplemented reset route;
that journey and visual parity remain in the later browser tranche.
Fresh integrated screenshots are saved under
`docs/screenshots/leptos-foundation/` (`840fd64b`).

Sync privacy slice of Task 5B: complete (`20348a31`, independent review found no
remaining findings). The change feed now scopes person rows to current grants
and medication changes to current snapshot visibility, including visible
health-event references. Medication deletion metadata is captured before
dependent records are removed; tombstones use current grants. The final
isolated Rails suite passed 6,102/0, RuboCop passed 1,892 files with no
offenses, docs build passed, and the integrated Rails-backed Rust privacy
contract passed 1/1. The Rust API remains absent, so Rust server parity and
the memory target are not yet verified. The first combined contract run exposed
a reused pause period in the new test's shared fixture; after the test resumed
both schedules before its cursor, the complete integrated corpus passed
228/0/12 across 31 groups.

API Task 7 audit at `670786ae`: the 228/0/12 Rails-backed run does not close
the baseline gate. The independent audit identified missing exact-set coverage
for health-history reports and an untested direct-write-purpose disk-token
route. The `show_hidden` OpenAPI/Rails discrepancy has the compatibility ruling
above. The 12 ignored
cases remain classified Rails defects or owned gaps, not green parity proof.
General browser-test writing waits for Task 7 reconciliation and handoff.

API Task 7 gap closures: the health-history report contract now compares its
complete eligible chronology with the paginated authorised health-event list
and independently requires four seeded episodes (`adfabfcb`, `f83624c8`,
`976ab37c`; independent review clean). A first combined run exposed health
events created by earlier contract groups; the corrected isolated branch
passed focused reports 7/7 and its full Rails-backed corpus 228/0/12 across
31 groups. The direct-write-purpose Active Storage disk-token PUT now has an
HTTP 404 and unchanged-blob read-back contract (`ec278e69`; independent
review clean), with focused Rails uploads 5 passed, 0 failed, 1 existing
ignored representation case. The final combined Rails-backed corpus passed
all 31 target groups on this branch, including the upload and report tests;
the 12 existing ignored cases remain. These additions do not close Task 7 or
turn ignored Rails defects into passing parity evidence.

Browser/PWA read-only inventory: first journeys are session boundaries,
parent medication assignment, and offline dose queue/replay. Source inspection
found the service worker caches `/offline` while the authenticated shell is
`/households/:slug/offline`; a real controlled offline navigation test must
establish the Rails baseline before this is treated as a confirmed runtime
defect or as a Rust parity requirement.

API Task 7 follow-up audit at `d32aafb6`: all 118 OpenAPI operation IDs have
exactly one matrix row and each referenced Rust contract file exists. This
does not prove operation behaviour coverage. The current 826 RSpec files
match the 826 inventory paths exactly, but 153 entries still carry
`review: true` and the inventory remains provisional. High-risk test gaps are
SMART patient-scoped OAuth issuance through consent into FHIR access, mobile
sync batch pause/close and dose
outcome branches, offline eligibility/future-dose rejection, MFA-sensitive
OAuth issuance, and support-access expiry/RLS. Independent API and retained
route audits found these against current Rails request specs. Test-only
contract slices now cover SMART issuance and offline dosing; sync actions
remain in review.
The browser/PWA tranche still waits for the reviewed API handoff.

RSpec inventory correction (`bdd85c61`, `e19f031a`; independent review clean):
all 826 current source paths remain present exactly once. Six classifications
were resolved with specific rationales; mixed authentication and 20 migration
specs were reclassified while retaining review flags for missing browser,
HTTP, or cutover evidence. The matrix now records API 79, browser 276, Rust
domain 302, operational 135, Rails-only 34, and 147 unresolved review flags.
Two finder JSON specs retain review flags because guidance and Open Food Facts
fields are not yet covered by portable HTTP assertions. This classification
work does not close the API test gate.

Offline contract gap closure (`c1ef57a1`; independent review clean): the
Rails-backed retained-route tests now check that inactive, cooling-down, and
expired schedules are excluded from the offline snapshot and that a future
queued dose returns 422 without changing stock or dose records. Focused Rails
passed 5/5. A production-style queued-write CSRF probe remains open.

SMART/FHIR contract gap closure (`390c3401`; independent review clean): a
nonce-scoped public client now exercises login, consent, PKCE, issuance of a
patient-bound bearer token, foreign-patient denial, refresh rotation, and
revocation. Focused Rails passed 1/1. Confidential-client authentication,
MFA-sensitive issuance, and external-provider success remain uncovered.
Both slices are tests against Rails; there is no corresponding Rust API yet.

Sync action contract gap closure (`f25389d9`; independent review clean):
isolated batches exercise schedule pause-period create/close, assignment
pause/resume/reorder, actor attribution, forged-input rejection, cached
replay, and late-operation rollback through public resource, change-feed,
and audit reads. Focused Rails passed 18/18. Cached create/close replay after
grant revocation and dose-outcome batch actions remain uncovered.

Finder JSON contract expansion (`4743b150`; independent review clean):
the web finder now checks available NHS guidance and result identity fields,
plus Open Food Facts supplement fields for barcode and text searches.
Focused Rails passed 7/7. Positive PIL URL, evidence prompts, and the
unavailable-guidance fallback remain inventory review gaps.

The integrated Rails-backed contract corpus passed all 32 target groups
after these additions. Twelve ordinary cases remain ignored with recorded
reasons; the separately enabled feature-disabled AI case passed. Rust
formatting, Clippy, documentation build, and inventory JSON validation pass.
This is Rails baseline evidence, not a passing Rust API implementation.

Platform support-access contract expansion (`f0b04db8`; independent review
clean): the web contract now checks held-household denial with the visible
authorisation alert, and an expired session's late DELETE creates one
session-specific expiry audit event rather than an explicit end event.
Focused Rails passed 4/4. The held no-row-write assertion remains in the
Rails request spec because the public UI has no support-session listing;
forced-RLS behaviour remains a PostgreSQL operational check.

Offline queued-write CSRF contract (`e619b690`; independent review clean):
the retained web tests now run with the production-style CSRF overlay.
Missing and wrong tokens return 401 with observer-visible takes and stock
unchanged; a valid rendered token creates one take and decrements stock once.
Focused Rails passed 6/6. The combined runner enables the same overlay for
the retained target. The subsequent integrated Rails-backed contract run
passed all 32 target groups with the 12 recorded ignored cases; the separate
feature-disabled AI case passed. API contract expansion is paused while the
first Rust medication read implementation proceeds.
