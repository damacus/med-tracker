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
- Preserve FHIR R4/SMART, MCP, signed attachments, reports, imports/exports and platform/support flows.
- Preserve complete browser operations, profile/security, devices and all five notification switches.
- Use Tera and daisyUI; export existing palettes from the official theme creator as committed CSS.
- Preserve local licensed fonts, palette names, `med-tracker-theme`, `med-tracker-appearance`,
  Light/Dark/System and live operating-system appearance changes.
- Preserve keyboard operation, focus, accessible validation, desktop/mobile layouts and offline replay.
- Preserve every inventoried job and recurring schedule with restart recovery and permission rechecks.
- Produce a final scratch image supporting server and worker on linux/amd64 and linux/arm64.
- Prove TLS, fonts/PDF, timezones, templates/assets and writable storage in the actual final image.
- Remove obsolete Axum/Leptos/Leptodon/Loom routing and Tasks after replacement acceptance.

## Delivery constraints

- Use maintained framework and security capabilities; no bespoke security protocol state machine.
- Use Task entry points for execution and Fish for shell steps. Fixtures have password `password`.
- Preserve source comments. Keep `AGENTS.md` and `agents.md` in sync.
- Keep one persistent implementation writer, one verification lane and Devin SWE-2 Max review.
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
- Every household operation has
  `async fn execute(db: &DatabaseConnection, scope: &HouseholdScope, input: Command)
  -> Result<Output, OperationError>` in its owning model module.
- An operation opens its transaction, establishes verified actor/household RLS context
  on that connection and rechecks current grants before reading or writing.
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
