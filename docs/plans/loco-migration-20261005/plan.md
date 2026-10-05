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
| 02 | [Persistence](slices/02-persistence.md) | P1 after 01 is locally verified and published; P2/P3 after 01 accepted | Schema parity, idempotent adoption, drift rejection, least privilege and rollback |
| 03 | [Identity](slices/03-identity.md) | 02 tenant/fixture interfaces accepted | Verified library choice, stored formats, browser security and OAuth/SMART |
| 04 | [Care operations](slices/04-care-operations.md) | 02 plus validated actors from 03 | Complete household, medication, treatment, dose and sync operations |
| 05 | [API and integrations](slices/05-api-integrations.md) | 04, relevant identity, and W1 before enqueue endpoints | API/native, FHIR/SMART/MCP, private data and integration parity |
| 06 | [Browser](slices/06-browser.md) | Relevant accepted 03–05 operations | Complete Tera journeys, existing themes, accessibility and offline replay |
| 07 | [Workers](slices/07-workers.md) | W1 immediately after 02; W2/W3 after their domain operations | All durable jobs, schedules, permissions, restart and drain |
| 08 | [Release](slices/08-release.md) | Every capability owner accepted | Both scratch runtimes, no legacy dependency, full rehearsal and rollback |

Execute one owned task at a time. The numbering is a navigation aid, not permission
to defer queue infrastructure: **02 → W1 → 03 → 04 → 05 → W2/W3 → 06 → 08**.
Identity dependency research may proceed independently, but there is one product writer.
Each browser journey family in B2 is independently reviewable and reportable.

While GitHub checks run on the immutable published setup commit, the writer may
start P1's isolated test-runner work. This does not accept setup or authorise schema
adoption. P2/P3 and persistence acceptance still require the accepted foundation.
Keep P1 changes separate from the published commit whose CI is being observed.

## Global constraints and review focus

The implementation specification is authoritative for shared constraints/interfaces.
Keep existing Task/Fish, comment-preservation, PostgreSQL 18 and fixture-password rules.
Do not implement security protocols or silently change client/credential compatibility.

Cross-slice review targets: tenant leakage (P3/C1), duplicate clinical writes (C3),
credential/session transitions (I1–I3), offline data leakage/replay (B3), and populated
rollback plus missing scratch assets (R1/R3). Each owning sub-plan assigns explicit tests.

## Progress and integration

Current state: foundation **unaccepted**. The font/context fixes, both complete
browser runners, corrective review, local Loco CI and documentation checks pass.
The verified setup is published at `9e54e930`; the prepared main merge and current
upstream client-tools lock update are being checked before publication. Hosted CI
still needs to pass. See `progress.md` for current evidence. Do not repeat completed
work or treat the separate legacy Rails diagnostic as Rust implementation.

Task reports live at `docs/plans/loco-migration-20261005/slices/<number>-report.md`;
the foundation keeps its existing detailed report. The coordinator owns `progress.md`
and `/private/tmp/medtracker-migration-status.html`, updated after each accepted slice
and on a material blocker. Report what shipped, what passed, what blocks delivery and
the next action. Keep follow-up issues minimal; migration defects stay in their slice.

Obtain independent task review, then full applicable slice checks and clean review
on the frozen commit. Commit/push only accepted changes with passing applicable gates.
Preserve existing local work when integrating documentation separately. Final broad
review covers all slices together before the deployment approval boundary.

This packet applies the [requested writing-plans skill](https://github.com/obra/superpowers/blob/main/skills/writing-plans/SKILL.md)
to the established plan location. The user-selected execution team takes precedence
over the skill's suggested fresh-agent methods; no new implementation dispatch occurs
as part of this planning update.

Approved direction: 5 October 2026. Implementation starts on `codex/loco-migration`.

## Outcome and constraints

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

## Devin review corrections

Devin CLI `swe-2-max` completed a supplied-plan review on 5 October 2026. Verdict:
conditionally sound, not implementation approval or runtime verification.

- Establish one schema owner after adoption: Loco. Preserve the Rails ledger;
  permit only additive compatible changes while rollback is retained. Compare
  normalized schema dumps with named exceptions, prove adoption idempotency and
  reject unrecognized drift. Rehearse Rails boot and journeys on the adopted DB.
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

Port existing light/dark palettes using the official daisyUI theme creator;
commit exported CSS directly. Preserve palette names, local licensed fonts,
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
