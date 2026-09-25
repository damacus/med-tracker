# Axum and Leptos parity port

Status: execution plan, 24 September 2026

Execution update, 25 September: subsequent user instructions authorized
bounded Rust implementation alongside the still-open parity work. Current
ownership and sequencing are in [team-charter.md](team-charter.md) and the
[mobile OAuth run](mobile-oauth-run-2026-09-25.md). These supersede the original
whole-baseline-before-implementation gate below. The historical plan remains
to explain the original scope; the ledger records current evidence and gaps.

## Outcome and decisions

Replace the Rails application with one Rust application process that serves the
versioned JSON API, server-rendered Leptos pages, PWA assets, and the remaining
required HTTP surfaces. Preserve accounts, medical records, authorization,
audit history, and externally observable behaviour. Existing sessions and API
tokens may require reauthentication at cutover.

The Rust API remains the public contract for Android, iOS, and browser-side
Leptos code. Server-rendered Leptos pages call the same application services
in process rather than making HTTP requests back to the same server. Both
adapters use the same authorization rules and response data types.

"All testing is ported" means every observable requirement in the Rails test
suite is either represented in the new black-box suites, represented by a Rust
domain test where no public route exposes it, or explicitly recorded as a
Rails-only implementation check with a reason. It does not mean translating
every RSpec example line for line. No Rust product behaviour beyond the
bounded Leptos foundation exception below is implemented until the API and
web parity inventories and their initial test suites are written, reviewed,
and run against Rails. Subsequent implementation follows Red-Green-Refactor
for each behaviour.

The application-process memory target is under 200 MB for the combined API,
web, and in-process job workload. PostgreSQL and supporting services are
outside that budget. Measure warm idle and representative requests separately;
do not claim the target from a framework benchmark or a process at startup.

Use an ORM-backed PostgreSQL 18 persistence layer. Before selecting the ORM,
compare stable Diesel Async and SeaORM on a representative authorized read and
an atomic medication stock update. Compare query clarity, schema checking,
transaction safety, integration-test setup, and measured application memory;
do not infer memory savings from the abstraction alone. Use parameterized raw
SQL only where the selected ORM cannot express a PostgreSQL-specific operation
clearly, with the owning domain and an integration test. Cover the chosen paths
with PostgreSQL integration tests. Keep API and browser parity tests at the
HTTP and UI boundaries, independent of the persistence implementation. Before
product implementation, review one shared foundation contract for workspace
crates, the ORM selection, auth context, response and error types, router
wiring, and migrations. Keep shared manifests, schema, router, and
cross-domain abstractions under one integration owner.

The ORM trial uses a Rails-migrated disposable PostgreSQL 18 database. Its read
is the medication API with manager, delegated, and denied household contexts,
including active person grants, linked schedules and direct assignments, and
creator-owned unlinked medications. Its write is the authorised absolute stock
adjustment: lock the medication row, preserve decimal precision, write the
audit version atomically, and exercise invalid, foreign, and concurrent
requests. Give both ORM candidates the same fixture, connection limit, Tokio
worker count, HTTP/serialization boundary, and warmup. Record query counts,
combined application-process RSS at warm idle and load, CPU time, and latency;
choose only after correctness and the under-200 MB budget are measured. Dose
recording adds advisory locks and stock callbacks and is a separate follow-up
trial, not a substitute for the initial comparison.

## Current source of truth

- `docs/api/openapi.v1.yaml` is the authoritative v1 mobile API contract. At
  planning time it contains 77 paths and 118 operations. Inventory each
  method/path, not merely each path.
- `config/routes.rb` also exposes web, OAuth, PWA, FHIR R4, MCP, uploads,
  reports, and platform administration routes. Include observable behaviour
  from these surfaces even where OpenAPI v1 does not describe them.
- `spec/requests/api/v1/`, other `spec/requests/`, `spec/system/`, and the
  relevant service, policy, model, job, and security specs supply cases and
  expected effects. The whole `spec/` tree contains 879 files in this
  checkout; a file inventory is required before any retirement claim.
- The existing PostgreSQL 18 data and portable export format are retained.
  Use isolated databases for both test targets. Never run parity writes
  against a live or shared database.

## Test architecture and completion evidence

Create a test-only Rust workspace under `rust/` first. Its API contract runner
must take a target base URL and exercise the real HTTP interface. The fixture
builder provisions disposable data through an isolated test database and
documented setup tasks; tests authenticate through public flows and verify
effects through public reads or the audit interface. It must not add a
test-only authentication bypass to the product server.

Create browser tests under `rust/tests/browser/` using Playwright. Run the same
journeys against Rails and Rust, including desktop and mobile viewports. Keep
stable semantic assertions for text, navigation, forms, errors, access, and
offline behaviour. Capture representative screenshots as review evidence;
pixel equality alone is not acceptance.

Maintain `rust/parity-matrix.md` with one row per OpenAPI operation and one row
per other externally visible behaviour. Record its Rails source spec or route,
new test, fixture, Rails result, Rust result, and disposition. Classify every
existing RSpec file as API, browser, Rust domain, operational, or Rails-only
implementation evidence. Every Rails-only exclusion needs a reason and
independent review. A test added to the matrix counts as ported only when it
has executed against Rails and produced the expected assertion result.

The test phase is complete when all v1 operations and other retained surfaces
have cases, all RSpec files are classified, the API and browser suites pass
against Rails, and expected Rust failures are recorded. Any mismatch in the
Rails baseline is investigated before using that case as a parity oracle.

## Ordered tranches

### Bounded Leptos foundation exception

Ruling: Start one isolated server-rendered Leptos foundation slice while API
contract work continues — the user explicitly requested an early UI start and
accepted an initially failing Rust browser smoke test — the cost if wrong is
reworking this shell after the shared router, API, and authentication
interfaces are agreed. The API and full browser parity gates still apply to
all later product behaviour; this exception proves only a public login-page
smoke path.

The foundation writer exclusively owns `rust/web/**`, including its own crate
manifest, source, and browser smoke test. Existing API writers retain
`rust/contract-tests/**`, `rust/parity-matrix.md`, API fixtures, and contract
task wiring. The coordinator owns this plan, the ledger, integration order,
and combined verification. Root workspace manifests and lockfiles, the shared
router, database/schema files, and API integration are reserved for a named
coordinator task after independent review. The UI writer must not edit them.

The foundation check ports the login-page assertions from
`spec/system/user_sessions_spec.rb` and the public heading/OIDC-absence
assertions from `spec/features/security/oidc_security_spec.rb` into one
target-independent browser smoke. It runs against Rails first, records the
absent Rust target failure, then runs against a minimal Axum-hosted Leptos SSR
shell. Authentication submission,
mobile navigation, PWA behaviour, and visual parity remain later browser
tasks. Review the isolated UI commit before serial integration, then run its
smoke and affected API contract checks in the integration worktree.

Each tranche has one writer, a file-based brief and report, an independent
requirements and quality review, and a ledger entry before acceptance. Use the
subagent-driven-development workflow for execution. There is no project team
charter in this checkout, so do not assume the persistent seats required by
team-development. Keep the branch reviewable as a dependent stack when code
starts to land; do not merge or deploy as part of the port.

| Tranche | User-visible result | Completion gate |
| --- | --- | --- |
| 0. Inventory and fixtures | A complete parity map and repeatable disposable Rails target | Every v1 operation and existing spec file is classified; fixture setup can run safely twice |
| 1. API contract tests | Executable tests for the public API and other retained HTTP integrations | Rails baseline green for each group; protected data, status, JSON, headers, side effects, and failure cases asserted |
| 2. Web and PWA tests | Executable tests for current journeys and offline care | Rails desktop/mobile baseline green; install, queue, reconnect, and permission-change cases covered |
| 3. Core Rust application | The same users and mobile clients can use the API | Rust API parity suite green, including authorization, dose, inventory, audit, sync, and imports |
| 4. Leptos web | Current web journeys work through server-rendered Rust pages | Rust browser suite green; accessibility and PWA behaviour match the baseline |
| 5. Operations and cutover rehearsal | Jobs, reports, storage, observability, and data transition are proven | Full parity matrix green; restored data and rollback rehearsed on disposable copies; memory measured |

Tranche 1 is subdivided into auth and envelopes; care and medication records;
sync and replay; administration and invitations; reports and portable data;
and uploads, notifications, lookup, FHIR, and MCP. These are sequential test
writing tasks with independent reviews, not parallel writers. A later tranche
may be split at an actual interface or verification boundary. Do not turn a
failed test into a changed expectation without checking current Rails behaviour
and the published contract.

## Quality and safety gates

- API contracts cover success, validation errors, authentication,
  authorization, cross-household isolation, pagination, ETags, idempotency,
  concurrent submissions, and transaction rollback where applicable.
- Record-dose cases verify timing, dose identity, stock mutation, immutable
  history, audit evidence, duplicate submission behaviour, and offline replay.
- Browser cases cover login, household selection, dashboard, medication and
  person management, scheduled and direct doses, reports, administration,
  responsive layout, keyboard access, and PWA offline recovery.
- Test the security-sensitive boundaries of OAuth/PKCE, sessions, tokens,
  permissions, attachments, exports, and audit records with denied cases.
- Preserve the existing v1 response contract and pinned native clients.
  Any intentional API change needs a separately reviewed versioning decision.
- Compare Rust and Rails with equivalent data, concurrency, warmup, database,
  and workload. Record combined application RSS/cgroup memory, latency and CPU
  separately; a lower idle figure does not prove load performance.
- No live database migration, reconciliation, rollout, account mutation,
  merge, or deployment is authorised by this plan.

## Planned paths and commands

The plan and briefs live under `docs/plans/axum-leptos-port/`. Tranche 0-2
writers own only `rust/parity-matrix.md`, `rust/contract-tests/`,
`rust/tests/browser/`, and the task/CI wiring needed to run those suites.
`app/`, `config/`, `db/`, `mobile/`, and `docs/api/openapi.v1.yaml` remain
read-only during test authoring unless a separately reviewed baseline defect
requires a change. Later product tranches own `rust/crates/` and their own
explicit briefs. Reports and reviews are uniquely named in this plan's
workspace; `ledger.md` records accepted tasks and rulings.

Existing checks: `task test TEST_FILE=...` for a focused Rails baseline,
`task test` only if Rails code changes, `task docs:build` for documentation,
and `git diff --check`. Tranche 0 must add task-wrapped commands named
`rust:contract:rails` and `rust:web:rails`; later tranches add corresponding
Rust-target commands and `rust:fmt`, `rust:clippy`, and `rust:test`. The brief
must give each command's exact arguments and target URL before it is used.
CI and local results are reported separately.

## First handoff

Execute [api-tests-brief.md](api-tests-brief.md) first. Write the API corpus
before [ui-tests-brief.md](ui-tests-brief.md). Only after both test phases
meet their gates should implementation briefs be cut from the parity matrix.
