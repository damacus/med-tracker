# MedTracker wholesale Loco migration

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

## Tranches and acceptance

1. **Foundation:** root Loco boot/routing/views, Rails relocation and command
   ownership, CI/path/tooling contracts, source-derived capability inventory and
   PR disposition ledger. Prove the standard framework configuration, including
   queue/protocol feasibility, before migrating sensitive behavior.
2. **Persistence and authentication:** non-destructive schema baseline adoption,
   independent fixtures, pooled transaction RLS context, credential/encryption
   compatibility, session policy and security middleware parity.
3. **API and integrations:** complete API journeys through shared domain
   operations, OpenAPI/native compatibility, FHIR/SMART, MCP, signed attachments
   and exports. Queue setup precedes any endpoint that enqueues work.
4. **Browser and themes:** all complete journeys in Tera/daisyUI, full profile,
   security and notifications, accessible interaction, explicit service-worker
   offline dose capture/replay and realtime behavior from the inventory.
5. **Workers:** all imports, reminders, notifications, review refresh, expiry and
   retention jobs with durable PostgreSQL queues, retry/idempotency, crash recovery,
   permission rechecks, schedules, graceful drain and observable failed work.
6. **Release:** remove superseded infrastructure/tasks, standalone no-Ruby Loco
   acceptance and independently runnable Rails rollback; static scratch images
   and runtime verification on linux/amd64 and linux/arm64.

Each tranche needs red-before-green behavior contracts, focused checks, a clean
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
#2389 (notifications), #2390 (profile), #2395 (fonts), #2397 (styles), #2399,
#2402 and #2403 (legacy UI infrastructure). Track each disposition and acceptance
evidence. Close superseded PRs only after captured work has replacement links.
Unrelated release PR #2381 is outside this migration.

The same scratch image supports separate server and worker invocations, with
distinct readiness/liveness and graceful shutdown. Verify actual runtime on both
architectures, not merely image manifest presence. Final acceptance includes
schema adoption/rollback, unauthorized roles, concurrent doses/replay, contract
clients, jobs/restarts, signed attachments, TLS/PDF/timezones and full UI journeys.
