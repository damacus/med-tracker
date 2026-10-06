# MedTracker wholesale Loco migration Implementation Plan

> For agentic workers: retain the user-selected persistent team and Devin SWE-2 Max reviewer.
> Read `team-charter.md` and the owning sub-plan before execution. Tasks use checkbox steps.

**Goal:** Replace the current application wholesale with maintainable Loco routing and complete product parity.
**Architecture:** Standard Loco controllers, SeaORM operations, Tera views and workers own the replacement.
Rails remains independently runnable under `rails/` until final rollback acceptance and authorised cutover.
**Tech Stack:** Loco 1.2.0, PostgreSQL 18, SeaORM, Tera, daisyUI and maintained protocol libraries.
**Spec:** [implementation-spec.md](implementation-spec.md).

## Implementation plans and dependency order

The existing high-level direction below remains the design context. The following
sub-plans are the execution packet, organised into slices rather than tranches.
Each task defines files, interfaces, a behavioural RED, implementation, GREEN and integration.
Proposed files and Tasks are labelled as new; their presence in a plan is not completion evidence.

| Slice | Sub-plan | Entry condition | Accepted outcome |
| --- | --- | --- | --- |
| 01 | [Foundation completion](slices/01-foundation.md) | Current published foundation and review fixes | Corrected image inputs, both runners, clean review and published CI |
| 02 | [Persistence](slices/02-persistence.md) | Locally verified and published foundation; owned database runner passes | Schema parity, idempotent adoption, drift rejection, least privilege and rollback |
| 03 | [Identity](slices/03-identity.md) | Locally verified tenant/fixture interfaces; library proof may run independently | Verified library choice, stored formats, browser security and OAuth/SMART |
| 04 | [Care operations](slices/04-care-operations.md) | Verified shared tenant/actor/error interfaces; production routes require identity | Complete household, medication, treatment, dose and sync operations |
| 05 | [API and integrations](slices/05-api-integrations.md) | 04, relevant identity, and W1 before enqueue endpoints | API/native, FHIR/SMART/MCP, private data and integration parity |
| 06 | [Browser](slices/06-browser.md) | Relevant locally verified operation; complete identity/workers required for acceptance | Complete Tera journeys, existing themes, accessibility and offline replay |
| 07 | [Workers](slices/07-workers.md) | W1 immediately after 02; W2/W3 after their domain operations | All durable jobs, schedules, permissions, restart and drain |
| 08 | [Release](slices/08-release.md) | Every capability owner accepted | Both scratch runtimes, no legacy dependency, full rehearsal and rollback |

## Execution correction from the retrospective

The eight sub-plans track complete scope, not eight serial approval stops.
Implementation depends on locally verified interfaces. Hosted CI and independent
review remain acceptance and merge gates; a queued or broken CI setup job does
not stop isolated implementation against an already verified dependency.
The foundation and owned runner are locally verified and published, so P2/P3
implementation can start while their immutable hosted checks run. This does not
accept foundation, persistence or release. Use only owned synthetic databases.

Preserve dependencies: establish tenant transactions and queue storage before
operations that require them. Reuse existing SeaORM entities, transactional care
operations, validation, projections and contract tests from `rust/api/`, adapting
HTTP boundaries to standard Loco. Do not recreate those operations merely to fit
a new directory plan. Replace the legacy router, browser dispatch, UI framework
and unsupported security flows as required by the approved architecture.

One writer owns the current shared persistence work. Separate future ownership
only where fixed interfaces and disjoint files make parallel implementation useful.
Devin reviews coherent capability changes and security/schema decisions. Routine
CI/tooling repairs receive focused tests and coordinator review, then join the
next capability review. Do not start a separate reviewer round trip for each
two-line setup correction or re-review non-blocking hypothetical nits.

Relocation ledgers and old-workspace checks are explicit `task migration:audit`
evidence, not part of everyday `task test` or `task ci`. Keep their original
records for relocation/rollback investigation. Do not regenerate snapshots to
permit ordinary implementation. Behaviour, security, schema and cleanup checks
remain in normal verification; complete product and rollback acceptance remain required.

Batch integration updates into coherent deliveries rather than pushing every
status edit and cancelling useful CI. Freeze review inputs and identify the exact
published commit being checked. Report completed behaviour, the actual blocker
and the next implementation action; retain detailed job records separately.

## Global constraints and review focus

The implementation specification is authoritative for shared constraints/interfaces.
Keep existing Task/Fish, comment-preservation, PostgreSQL 18 and fixture-password rules.
Do not implement security protocols or silently change client/credential compatibility.

Cross-slice review targets: tenant leakage (P3/C1), duplicate clinical writes (C3),
credential/session transitions (I1–I3), offline data leakage/replay (B3), and populated
rollback plus missing scratch assets (R1/R3). Each owning sub-plan assigns explicit tests.

## Progress and integration

Foundation is accepted. Published deliveries include signed-in care, medication
management, OAuth, household administration and supported passkeys. Account setup,
invitations, stored API credentials, medication reads and dosage management have
clean independent review; their final combined verification is running. The other
migration areas remain unfinished. Use [progress.md](progress.md) and the readiness
report for the current verification and publication state.
Do not repeat completed checks or treat Rails diagnostics as Rust implementation.

Task reports live at `docs/plans/loco-migration-20261005/slices/<number>-report.md`;
the foundation keeps its existing detailed report. The coordinator owns `progress.md`
and `/private/tmp/medtracker-migration-status.html`, updated after each accepted slice
and on a material blocker. Report what shipped, what passed, what blocks delivery and
the next action. Keep follow-up issues minimal; migration defects stay in their slice.

Obtain independent review for coherent capability changes, then full applicable
checks and clean review on the frozen commit. Publish locally verified integration
commits; acceptance and merge additionally require the applicable hosted gates.
Preserve existing local work when integrating documentation separately. Final broad
review covers all slices together before the deployment approval boundary.

This packet applies the [requested writing-plans skill](https://github.com/obra/superpowers/blob/main/skills/writing-plans/SKILL.md)
to the established plan location. The user-selected execution team takes precedence
over the skill's suggested fresh-agent methods; no new implementation dispatch occurs
as part of this planning update.

Approved direction: 5 October 2026. Implementation starts on `codex/loco-migration`.

## Outcome and constraints

The 6 October authorisation correction requires a maintained policy library.
The owner selected embedded Cedar on 6 October. Integration requires an executable
proof of the current allow/deny matrix and both scratch targets. Replace bespoke
permission decisions through one fail-closed engine boundary. Preserve trusted
current SeaORM data, locking/rechecks, list scoping and PostgreSQL RLS; do not create
a second mutable grants store or permission cache. The current reviewed capability
delivery is an intermediate publication, not completion of this requirement.

Make Loco the root application using its standard routing, controllers, SeaORM,
Tera views, initializers and workers. Move Rails to `rails/` as an independently
runnable reference and rollback application. Root Task commands belong to Loco;
Rails commands exist only under `rails:`. Remove the final legacy Axum router,
Leptos, Leptodon, Loom and browser-to-API in-process dispatch. API and browser
actions call the same authorized domain operations.

Preserve all product capabilities: API v1 and native pinned contracts, household
permissions, capacity rules, dose transactions and stock/audit/replay/sync,
OAuth/PKCE, MFA/passkeys, signed attachments, profile, push/reminders, exports,
FHIR R4/SMART, MCP, admin/support, PWA/offline and background jobs. Inventory
actual source and open PRs; the old parity matrix is not authoritative.

Use mature framework/security dependencies. Never reimplement cryptography or
protocols to simplify migration. PostgreSQL 18 remains the database. Person kinds
remain adult=0, minor=1, dependent_adult=2; the latter two lack capacity.

No live migration, deployment or PR merge is part of implementation. Keep pending
work visible rather than declaring a skeleton production ready. One migration
branch/replacement PR is the final integration vehicle.

## Scope areas and acceptance

The eight sub-plans above replace the earlier six-phase sequence. Persistence
and identity are separate decisions; shared care operations have their own owner
and acceptance boundary. Queue provisioning is an explicit early prerequisite.
All originally approved product and release requirements remain in the specification.

Each slice needs red-before-green behavior contracts, focused checks, a clean
independent review and accepted evidence before completion. Intermediate commits
may be integration waypoints; none is a partial cutover.

## Approved cutover decisions

On 6 October 2026, the project owner approved dropping historical passkey algorithms
unsupported by the selected maintained Rust WebAuthn library, including PS256.
Affected users may need to reauthenticate and register replacement passkeys. This
is an intentional compatibility break, not unfinished compatibility research.
Do not add custom verification or another provider to preserve those algorithms.

Do not bulk-delete old credential rows during migration; retain them for Rails
rollback until their owner explicitly removes them. Supported keys must remain usable
on mixed-key accounts. Verify clear replacement messages and supported recovery
and re-enrolment, including unsupported-only accounts, without silently bypassing
MFA. Counting affected production accounts is not a gate: the owner accepts
invalidating unsupported historical passkeys. Record user instructions and prove
the transition with synthetic accounts.

The owner also approved a saved-state rollback on 6 October 2026. Preserve an
independent pre-cutover dump or frozen replica; prove restoration and Rails
operation on that saved state. A replica stops following before Loco writes.
All new data created during the Loco period may be dropped or ignored on rollback.
Rails compatibility with Loco-created data and lossless rollback are not required.

Existing sessions and tokens will be invalidated. Prove the mechanism with synthetic
credentials; preserve fresh sign-in and MFA, rather than historical continuity.
The owner also permits clearing passwords and disabling existing MFA enrolments,
requiring secure reset/onboarding before access. Historical hash/enrolment continuity
is excluded; future MFA requirements are a separate decision. Prioritise a maintained
full account lifecycle replacement for Rodauth inside MedTracker. Separate identity
services, including Rauthy, are rejected. Better Auth RS is preferred, including
OrganizationPlugin for households with multiple memberships. Prove the actually
used Rodauth feature mapping before integration; proposed drops need an owner
decision. New custom reset/settings work is paused.
Old download links may expire; preserve files/data and issue new authorised links.
Historical system export formats and pre-cutover offline queues need not migrate;
new exports and future offline replay remain required. PDF reports must match the
existing appearance. Inspect the renderer, fonts and retained Rust work first and
verify the selected implementation in scratch.

Use daisyUI browser pages with the same colour schemes and clear Loco routes.
Required information, actions and accessibility remain; exact Rails pixels, CSS
values/geometry, theme-export fidelity and historical browser URLs do not.
Historical push subscriptions need not survive; users may enable notifications
again, with normal future push behaviour and preferences preserved.

The core first production release excludes FHIR/SMART, MCP, AI and external medication
lookup. These are deferred capabilities within the full migration objective, not
permanently removed work. Exact AI/lookup provider configuration parity is unnecessary.
Core medication entry and care remain required. Report the core production milestone
separately; the active goal completes only when every remaining migration capability
is implemented and verified.

The subsequent first-production scope answers require automated NHS dm+d import/
reconciliation and scanner, medication review generation/background refresh,
complete platform administration, temporary time-limited support access, household
export/closure/retention holds/permanent deletion, avatar uploads, complete device/
session management and automatic live dose/stock updates. Catalogue/scanner and
reviews are core; optional AI/provider lookup deferral does not exclude them.

Browser offline capture/replay, acceptance of both native apps before cutover and
portable imports are deferred from the first production gate. They remain in the
complete migration goal. Public API correctness remains required. Record genuine
dependencies of required outcomes as blockers rather than quietly deferring them.

Pending or failed Rails jobs may be lost; do not require old queue transfer or
individual delivery reconciliation. Correct Loco workers and schedules remain
required. Scale Rails servers to zero and stop its separate workers and schedulers
before Loco starts; stop Loco before restoring Rails. These decisions change the
cutover design, not the separate live-action approval boundary.
See [the deployment decision](../../deployment.md#approved-cutover-decision-unsupported-passkeys)
and [the identity transition requirements](slices/03-identity.md#approved-passkey-transition-6-october-2026).
The approval changes the compatibility requirement; it does not waive verification
of the transition or authorise production cutover.

## Devin review corrections

Devin CLI `swe-2-max` completed a supplied-plan review on 5 October 2026. Verdict:
conditionally sound, not implementation approval or runtime verification.

- Establish one schema owner after adoption: Loco. Preserve existing records and
  the Rails ledger. Rollback restores the independent pre-cutover state; Rails
  compatibility with the Loco-era schema is not a requirement. Compare
  normalized schema dumps with named exceptions, prove adoption idempotency and
  reject unrecognized drift. Rehearse Rails boot and journeys on the restored
  pre-cutover database.
- Inventory real password hashes/pepper, TOTP and encrypted-column formats,
  session/token formats and signed attachments. Prove migrated credentials and
  existing client tokens work; record unavoidable session/URL changes explicitly
  before implementation. Never invent a compatible cryptographic implementation.
- Keep application RLS enabled in HTTP and workers. Set actor/household context
  inside the same transaction/connection as queries; prohibit silent worker
  bypass and test pooled-connection reuse and negative permissions.
- Gate CSRF, CSP, cookies/SameSite, session expiry/revocation, secure headers and
  rate limiting alongside authentication. Audit raw SQL and public asset access.
- Validate FHIR/SMART server, MCP, WebAuthn and Web Push library support early.
  An unsupported requirement is a documented gate, not permission to omit it.
- Use Loco's documented PostgreSQL BackgroundQueue first. Verify retry/reaper,
  schedules and shutdown behavior against the pinned version; do not add an
  alternative queue merely on the reviewer's unverified capability assumption.
- Keep offline writes/replay explicit. Test browser/API authorization together.
  Pin the inventory to source revision and reconcile upstream changes before
  acceptance rather than imposing an unapproved feature freeze.
- Include timezone data (embedded maintained library or zoneinfo), TLS CA roots,
  templates/assets/fonts and writable storage in scratch runtime evidence.

## UI and task contract

Use daisyUI light/dark themes with the same colour schemes; fresh themes are allowed.
Commit CSS directly. Preserve usable theme preferences, local licensed fonts,
`med-tracker-theme` and `med-tracker-appearance` preferences and Light/Dark/System
including live OS changes. Use standard daisyUI component geometry. Application
JavaScript owns keyboard tabs/dialogs, focus and validation; CSS is not sufficient
for interactive or offline behavior. Verify desktop/mobile UI and screenshots.

Root tasks cover dev/build/test/check/lint/fmt/routes/db:migrate/db:seed/worker/
browser/release-image/ci. Keep useful docs/contracts/native/client generation
tasks; delete superseded aliases after replacement. Fixtures use password
`password`. Source comments are preserved without new/removal edits.

## PR salvage and release

Capture useful behavior and source from PRs #2419 (schema), #2418 (scratch),
PRs #2389 (notifications), #2390 (profile), #2395 (fonts), #2397 (styles), #2399,
plus #2402 and #2403 (legacy UI infrastructure). Track each disposition and acceptance
evidence. Close superseded PRs only after captured work has replacement links.
Unrelated release PR #2381 is outside this migration.

The same scratch image supports separate server and worker invocations, with
distinct readiness/liveness and graceful shutdown. Verify actual runtime on both
architectures, not merely image manifest presence. Final acceptance includes
schema adoption/rollback, unauthorized roles, concurrent doses/replay, contract
clients, jobs/restarts, signed attachments, TLS/PDF/timezones and full UI journeys.
Replace the foundation's development-only production configuration before release:
require externally supplied credentials, deployment binding/host settings and
appropriate connection limits. The foundation release task remains gated until
these settings and the complete runtime contracts are accepted.
