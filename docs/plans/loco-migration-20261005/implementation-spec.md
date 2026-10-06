# Loco migration implementation specification

This specification consolidates the approved direction and subsequent user corrections.
It does not accept the foundation or authorise a live migration, merge or deployment.

## Product outcome

Loco 1.2.0 becomes the complete root application. Standard Loco controllers expose
obvious routes; HTTP controllers, Tera pages, integrations and workers reuse the same
authorised operations. Rails remains runnable under `rails/` for reference and rollback.
The replacement must preserve all capabilities in [capability-inventory.md](capability-inventory.md)
and the revision-pinned machine inventory. A working skeleton is insufficient.

## Required behaviour

- PostgreSQL 18; adopt the existing schema without destroying records or Rails ledger provenance.
- Preserve identifiers, foreign keys, stored credential bytes, encrypted formats and audit history.
- Person values: `adult:0`, `minor:1`, `dependent_adult:2`; the latter two have `has_capacity:false`.
- Enforce household boundaries and admin, clinician, self, carer, parent and denied access.
- Keep dose, stock and audit writes transactional; preserve concurrency, conflict and replay semantics.
- Preserve API v1, the authoritative `docs/api/openapi.v1.yaml`, and pinned native clients.
- Preserve OAuth server/PKCE, refresh/revocation, MFA/passkeys, session policy and security middleware.
- Apply the approved 6 October passkey transition: drop algorithms unsupported by
  the maintained library, retain rows for Rails rollback, and offer explicit
  reauthentication/replacement through existing verified flows. Supported keys
  remain usable; unsupported keys never cause a silent MFA bypass.
- Preserve FHIR R4/SMART, MCP, signed attachments, reports, imports/exports and platform/support flows.
- Preserve complete browser operations, profile/security, devices and all five notification switches.
- Use Tera and daisyUI with the same colour schemes; fresh themes are allowed.
- Preserve local licensed fonts, palette names, `med-tracker-theme`, `med-tracker-appearance`,
  Light/Dark/System and live operating-system appearance changes.
- Preserve keyboard operation, focus, accessible validation, desktop/mobile layouts and offline replay.
- Preserve every inventoried job and recurring schedule with restart recovery and permission rechecks.
- Produce a final scratch image supporting server and worker on linux/amd64 and linux/arm64.
- Prove TLS, fonts/PDF, timezones, templates/assets and writable storage in the actual final image.
- Remove obsolete Axum/Leptos/Leptodon/Loom routing and Tasks after replacement acceptance.

## Approved cutover acceptance — 6 October 2026

Unsupported historical passkeys may be invalidated; counting affected production
accounts must not block migration. Synthetic recovery, replacement and mixed-key
tests remain required, with authentication and MFA enforced.

Pending or failed Rails jobs may be lost at cutover. Old queue transfer and
delivery-by-delivery reconciliation are excluded from acceptance. All Loco jobs
and schedules still require their normal correctness and recovery evidence.

Rollback restores an independently preserved pre-cutover database dump or frozen
replica and restarts Rails against that state. A replica must stop following before
Loco writes. Prove that the saved state restores and Rails operates on it. The owner
accepts dropping or ignoring all data created during the Loco period; Rails
compatibility with Loco-created data and lossless post-cutover rollback are excluded.
Adoption must still preserve existing records, credentials and audit history.

Existing sessions and tokens will be invalidated; prove synthetic invalidation
and fresh authentication without an MFA bypass. Historical session/token continuity,
old download signatures, historical system export formats and pre-cutover offline
queues are excluded. Preserve underlying files/data and account credentials; new
authorised downloads, Loco exports and future offline replay remain required.

The owner subsequently permits clearing passwords and disabling existing MFA
enrolments, with secure usable reset/onboarding before clinical access. Historical
password hashes and MFA enrolments need not remain usable in Loco. Preserve rollback
credentials in the independent saved state. This does not claim an incident or
authorise live mutations. Passkeys are the primary passwordless login: the browser
authenticator prompt must not require username/password entry. TOTP remains a
separate MFA capability. Recovery codes are required for every login method,
including passkey-only accounts, independently of TOTP enrolment.
Prioritise a feature-complete maintained account-lifecycle implementation inside
MedTracker. Rauthy and separate identity services are rejected. Better Auth RS is
selected, including OrganizationPlugin for households and multiple memberships.
Implement the library through normal account and household journey tests. Check
transaction, recovery and closure behaviour as part of that implementation, rather
than creating a separate proof gate before coding. A concrete library limitation
that changes an agreed requirement needs an explicit owner decision. Do not extend
custom lifecycle by default.

Use daisyUI browser pages and clear Loco routes. Keep the same colour schemes,
required information/actions and accessibility. Exact Rails pixels, CSS geometry,
theme-export fidelity and historical browser URLs are excluded. Push subscriptions
may require re-enrolment; future push behaviour/preferences remain required.
PDF reports must match the existing appearance. Use the selected `sghtmltopdf`
renderer, retained report templates/fonts and a verified final scratch runtime.
Use `aws-sdk-s3` with the existing RustFS service for shared persistent files so
multiple application replicas do not depend on one replica's local files.
When enabled, Gravatar is the primary avatar source. Optional uploaded avatars
use the maintained `image` crate for basic resizing, accept PNG/JPEG/WebP and
retain the existing 5 MB limit. No new output-size requirement is approved.

The first production gate excludes FHIR/SMART, MCP, AI and external medication
lookup. These remain in the eventual complete migration objective. Exact external
AI/lookup provider configuration parity is unnecessary; core medication entry and
care remain required. Report core release readiness and full migration completion
separately.

The owner's subsequent first-production answers refine that boundary:

| Required before first production | Deferred from first production; retained in full goal |
| --- | --- |
| Automated NHS dm+d import/reconciliation and scanner | Browser offline capture/replay |
| Medication review generation and background refresh | Acceptance of both native apps before cutover |
| Complete platform administration and time-limited support access | Portable imports |
| Household export, closure, retention holds and permanent deletion | FHIR/SMART, MCP and optional AI/provider lookup work |
| Avatar uploads and complete device/session management | Historical compatibility explicitly excluded above |
| Automatic live dose/stock updates | |

Catalogue/scanner and medication review outcomes are core requirements. Optional
AI/lookup deferral must not omit them. Identify an unresolved dependency explicitly
rather than deferring a required outcome. Core public API correctness remains
required even though both native applications need not be accepted before cutover.

Stop Rails servers, separately deployed workers and schedulers before starting
Loco; stop Loco before restoring Rails. Prove exclusive write ownership in the
rehearsal. These decisions do not authorise live data access, destructive actions,
production shutdown, merging or deployment.

## Delivery constraints

The owner requires library-backed authorisation rather than an application-owned
Pundit replacement. The owner explicitly selected the embedded `cedar-policy` crate
on 6 October. Prove the current permission matrix before accepting integration;
do not reopen engine selection without a demonstrated limitation. Household-scoped roles,
person view/record/manage, revoked/expired grants, capacity, time-limited support
and list visibility must pass. Reject evaluation diagnostics as well as explicit
deny. Trusted SeaORM reads, transaction rechecks/locking, SQL list scoping and RLS
remain; no second grants store or synchronised permission cache. Verify build/runtime
impact and both scratch targets. Integrate at the next safe source boundary.

- Use maintained framework and security capabilities; no bespoke security protocol state machine.
- Use Task entry points for execution and Fish for shell steps. Fixtures have password `password`.
- Preserve source comments. Keep `AGENTS.md` and `agents.md` in sync.
- Keep the persistent team with disjoint writer ownership, one verification lane and Devin SWE-2 Max review.
- Root owns planning, integration, commits and push; no overlapping source ownership.
- Verification uses owned disposable resources, explicit endpoints and retained cleanup evidence.
- Existing volumes and unrelated work are preserved. No live data is copied into test fixtures.
- Migration defects are fixed within their slice. Follow-up issues cover independent remaining work only.
- Update the HTML report after each accepted slice, and when a material blocker changes.
- All slices feed one wholesale migration and final cutover; there is no partial production cutover.

## Execution and acceptance contract

The linked writing-plans skill supplies the task structure. The user-selected persistent
team and Devin reviewer override its default fresh-agent execution suggestions.
Read this specification, the owning sub-plan, and [team-charter.md](team-charter.md).

For every task: demonstrate a behavioural failure, implement the smallest correction,
run focused checks, review the task diff independently, then integrate it. At the end
of a slice, freeze the commit, run applicable complete gates, obtain clean slice review,
push and verify the published head. Do not rerun unaffected expensive suites after a
documentation change. Do not weaken a required check to escape a failure.

The coordinator records `slice`, `commit`, `changed paths`, `checks and results`,
`review verdict`, `published head` and `next action` in `progress.md`. The human-facing
report is `/private/tmp/medtracker-migration-status.html`; the requested Markdown copy
is `/private/tmp/medtracker-2450-review-status.md`. Update both from the same evidence.

After two evidence-based attempts at the same unexplained failure, stop repeating
the experiment and have the coordinator localise it. Escalate a concrete unresolved
architecture/security decision to Astra when needed. Do not escalate an explained
path defect solely because it appeared late.

## Shared interfaces to establish in persistence

These are proposed interfaces, not existing source. Persistence owns their definitions;
later plans must consume these names or update all consuming plans before dispatch.

- `src/models/access.rs`: `Actor { account_id: i64 }` and
  `HouseholdScope { actor: Actor, household_id: i64, request_id: String }`.
- `src/models/errors.rs`: `OperationError` variants `Unauthenticated`, `Forbidden`,
  `NotFound`, `Validation`, `Conflict`, `Unavailable`; preserve public HTTP error details
  through the API plan's mapping rather than exposing database failures.
- Household operations use named methods in their owning model modules, sharing
  the existing `TenantTransaction` and `OperationError` interfaces. API, browser and
  worker adapters call those same methods. Do not add a `Command` dispatcher or an
  `execute` wrapper solely to satisfy the proposed migration interface.
- Establish verified actor/household RLS context on the transaction connection and
  recheck current grants before reading or writing. Keep related writes, audit and
  completion in that transaction; callers must not bypass this boundary.
- Library-owned credential validation produces `Actor`; it does not grant permission
  to a household or person. Workers revalidate the stored actor's current permissions.
- `scripts/migration/run-slice.mjs` owns disposable PostgreSQL 18, explicit test URLs,
  child process shutdown and cleanup, using the existing owned-database helper.
- New public `task slice:test TARGET=<test-binary> FILTER=<test-name>` runs root
  `cargo test --locked --test <test-binary> <test-name>` inside that owned environment.
  `FILTER` is optional. The task is created and tested in persistence before later use.
- New `task slice:test-runner` runs `node --test scripts/migration/run-slice.test.mjs`
  and verifies the owned test runner independently of application integration tests.
- New `task slice:contracts GROUP=<group>` and `task slice:browser GROUP=<group>`
  target the actual Loco server and representative isolated fixtures; their definitions
  are acceptance infrastructure owned by the API and browser plans respectively.

Do not point new acceptance Tasks at legacy Rust by accident. A missing Task is
an unimplemented plan step, not evidence that its listed checks already pass.

## Source reconciliation and self-review

Pin baseline `838e79da76aceed1155d047dae038e1bd2bad5f1` and retained PR refs from
`pr-inputs.md`. Reconcile newer upstream changes before each affected slice; do not
impose an unapproved feature freeze. Normalised catalog differences require names
and reasons. Protocol or client compatibility changes require explicit decisions.

Coverage map: foundation owns routing/tooling preservation; persistence owns schema/RLS;
identity owns authentication/security; care owns clinical writes and household permissions;
API owns public/native/integration contracts; browser owns complete UI/themes/offline;
workers own durable delivery; release owns scratch/rollback/legacy removal.
Every row in the capability inventory must have an accepted slice and evidence before release.
