# Clinical-data import from MedTracker 0.5

## Objective and acceptance

Provide a household Administration journey to upload a MedTracker 0.5 household
export, map people, inspect a dry run, acknowledge omissions, explicitly confirm,
and inspect the completed report. Reports must also be available through an
authenticated and authorised JSON API. Expected actions and verified destination
records AND relationships must be distinct. Group by person and retain a shared
inventory section, totals, reasons, discrepancies and destination links.

Use a new synthetic 0.5 canary in Home Ops because the former Rails canary was
removed and the existing canary now runs Rust. Export the seeded source and
compare Home Ops/seed expectations, actual export, preparation decisions and
actual imported results in a new disposable 0.6 database. Retain evidence and
give Dan usable sign-in and visual inspection instructions. Report exact access
or technical blockers; do not substitute synthetic unit fixtures for this real
acceptance exercise or claim universal compatibility.

## Global Constraints

- Terra implements. Separate worktrees permit parallel writers, explicitly
  overriding the SDD skill's default single-writer scheduling rule. No worker
  dispatches other agents. Controller owns integration, reviews and delivery.
- New 0.6 credentials are separate from imported people. Explicit adult mappings
  require current consent; never match identity solely by email or replace the
  destination account/person identity. No password, passkey or session import.
- Require current household/clinical permission, RLS and fresh attributable
  guardian declarations. Source carer relationships never confer authority.
- Unsupported items require item-level reasons and explicit acknowledgement.
  Clinical dependencies, identity conflicts and insufficient authority block
  apply. Obsolete person-location links may be deliberately omitted; medicine
  storage and immutable taken-from location links must remain intact.
- Preserve source values, dates, Decimal units/quantities, original timestamps,
  schedules, assignments, dose history, stock and supported associations. Inspect
  attachments and associated records explicitly. Do not silently discard data.
- Restore never repeats stock deduction or sends live notifications/reminders.
  Apply is transactional and replay-safe; verify actual relationships before
  committing a successful report. A failed apply retains no partial clinical data.
- Keep existing portable API contracts compatible. New import APIs are under
  /api/v1/households/{household_id}/data_imports. Reuse preparation/import internals.
- Confirmations bind reviewed source bytes/hash, mappings, declarations and
  omissions. Recheck access and destination state; changed expectations require
  another preview. Never persist passphrases or log clinical payloads/secrets.
- Follow current root AGENTS.md, accepted ADRs and Rust skills. SeaORM access,
  typed domain interfaces, no Python, Fish/RTK, published dependencies. Current
  baseline is refreshed main (Loco 1.3.1); do not restore superseded scheduler pins.
- Meaningful test-first checks for executable changes. No unit test merely to
  represent canary acceptance. Required final task ci and documentation checks.
- Never touch production data or deploy 0.6 to production. User authorised a new
  isolated seeded 0.5 canary. External deployment/publication is controller-owned;
  workers prepare concrete reviewed changes and never merge/push/deploy themselves.
- Preserve all unrelated worktrees and dirty working copies. One heavy CI/browser
  process at a time; isolated disposable databases and service ports per worker.

## Coordination and interfaces

Contract baseline is reviewed first, alongside independent canary preparation.
Then Tasks 3, 4 and 5 run concurrently in separate worktrees. Task 4 exclusively
owns importer internals, persistence, migrations, entity registries, API/OpenAPI
and shared API client. Task 3 owns clinical_transfer and its archive adapter.
Task 5 owns administration controllers/routes/navigation/views/translations.
Controller owns shared dependency/lock files and test-suite registration.
Contract changes require controller coordination, never cross-owner edits.

## Task 1: Freeze shared import contracts

Own new src/models/data_imports/contracts.rs, its parent module and initial model
registration only. Define typed source preparation, mappings, acknowledgements,
preview/actual report results and lifecycle structures, plus minimal service
signatures or documented signatures for preview/list/detail/confirm. Do not add
fake runtime success implementations. Use domain IDs, enum codes/states and
explicit expected-versus-actual outcomes. Source identities include source
provenance; destination identifiers and links remain distinct. Relationship
items describe endpoints and expected/verified status; known unsupported fields
and omissions have machine-readable reasons. Freeze report examples for UI work.
Keep JSON at transport/document boundaries, no internal Value round-trip APIs.
Design the narrow consent input around existing proven application authority;
never assume a manager can approve another adult's identity or clinical edits.
Inspect existing contracts before adding abstractions. Provide the signatures,
examples and exact ownership notes in the task report for downstream workers.
Run focused native validation; commit and self-review.

## Task 2: Prepare an isolated 0.5 Home Ops canary

Own only new medtracker-05-canary manifests, required current-main registration
and concise operating instructions in a separate Home Ops checkout. Read its
current AGENTS.md and established deployment patterns. Use namespace
medtracker-05-canary and hostname med-tracker-05-canary.damacus.io. Pin
ghcr.io/damacus/med-tracker:0.5.35@sha256:f47f317657236675427f6b60761858f8203c0f44c1755ea16b5f926058c73925.
Use isolated database, secrets and disk storage, DEMO_MODE=true, no production
restores/copies and no live outbound notifications. Reuse the committed Rails
weekly-demo-v1 seed and guarded seeding/reset machinery for this new environment.
Baseline: Alex Demo and Casey Demo adults, Rowan Demo child; one household,
two medicines, one daily schedule, one as-needed assignment, two takes, one
storage location and three obsolete person-location memberships. Preserve
source care relationship and omitted field expectations in evidence. Capture
relative dates at the actual seed time, never invent static dates.
Render/schema/security-check manifests using current Home Ops task tooling;
do not add tests that repeat YAML. Document deployment, one-time seed, verification
and complete hosted household ZIP export commands. No push/merge/deploy by worker;
controller handles reviewed deployment and source export. Never reset existing
environments. Write report with exact blockers and requirements.

## Task 3: Implement household ZIP preparation

Own src/models/clinical_transfer.rs and focused new archive/preparation modules
and their tests; use Task 1 contracts. Support the complete household export ZIP
with portable.json and manifest.json, reusing existing validation/crypto and
limits. Validate paths, duplicate entries/keys, counts, references, sizes and
precision. Keep original export unchanged and record hashes/transforms.
Replace blanket rejection of obsolete person-location links in the NEW adapter
with itemised acknowledged omissions, preserving existing strict tooling
behaviour unless deliberately extended and documented. Inventory attachments
and unsupported fields individually; do not extract or execute unsafe content.
Do not claim omitted exporter fields were imported. Unsupported dependencies
block rather than silently orphaning supported clinical records. Return typed
prepared input and inventory for the backend; no database writes or authority
creation. Use existing synthetic fixtures first, then inspect actual source
archive when available. Test-first focused behavioural validation and self-review.

## Task 4: Implement import backend and persistent reports

Own data_imports service/persistence, existing care portable-import changes,
explicit account-person mapping/consent, migration/entities/RLS, JSON controllers,
OpenAPI and shared API client. Consume Task 1 contracts and Task 3 adapter.
Persist household-scoped import runs/items, source mapping provenance and
reviewed decisions. Never rewrite destination canonical identities. Recheck
adult consent and live guardian/clinical authority at mutation boundaries.
Preview is non-mutating for clinical data; report metadata may persist. Add
preview creation, list/detail and explicit confirmation routes under agreed
data_imports path. Bind confirmation to reviewed input and invalidate changed
expectations. Serialize concurrent apply and make retry/replay safe.
Clinical writes, identity mappings and successful verified report commit together.
Verify actual destination rows AND links (foreign keys/junction rows), source
values, quantities/timestamps and omissions, not just endpoint existence.
On failure roll back clinical writes then retain safe failure metadata separately;
never label uncertain/rolled-back outcomes as successfully created. Do not deduct
historical stock again or enqueue live notifications. Apply existing access/RLS
to report reads, including person-specific restrictions and revocation.
Focused failure-first acceptance tests cover mapping/consent, authority, omissions,
staleness, concurrency, rollback, relationships and replay. Coordinate test-suite
registration with controller. Commit, self-review and report remaining risks.

## Task 5: Implement administration screens

Own new administration import controller/routes, navigation, templates, translated
copy/presentation helpers and browser scenarios. Use reviewed Task 1 report
examples to implement presentation in parallel; no fake service success routes.
Add Import data from another system, offering only MedTracker 0.5. Implement
upload, explicit adult mapping and consent status, required declarations,
person-grouped preview, reasons/conflicts/omissions, acknowledgement, confirmation
and final report. Link authorised readers to actual destination records. Retain
CSRF, scope binding, accessibility, mobile/desktop usability and existing styles.
Never persist passphrases; request again when required. Wire live services after
Task 4 integration. Verify screenshots under docs/screenshots and actual keyboard
journey, including denied, invalid, stale, rollback and completed states.
Own scoped browser checks, but reserve browser runner with controller.

## Task 6: Real canary acceptance and final handoff

Run only after reviewed source deployment/export and app integration. Export the
actual seeded 0.5 canary using its complete household exporter. Record seed/image
revision, hashes, dates and inventory, checking for non-synthetic additions.
Use new disposable 0.6 database, separately created fixture credentials and
explicit adult mappings/current consent and guardian declarations. Preserve
the source and existing deployments. Capture dry-run JSON, confirmed report JSON,
authenticated destination query evidence and desktop/mobile screenshots.
Compare expected/setup, exported, prepared and imported stages per record AND
relationship; identify exporter omissions (including expiry/professional-title/
care-relationship fields absent from serializer) rather than inventing data.
Exercise replay, unchanged stock after historical restore and everyday workflows.
Give Dan destination URL, separate sign-in steps, report/API URL and checks for
Rowan's medicines, daily schedule, as-needed treatment, history and stock.
Report exact concrete blockers and retained evidence if source access or technical
conditions prevent completion. No unit test for this acceptance exercise.
Run final task ci, docs build/diff checks and independent final review. Publish
reviewed independent slices via controller; no implicit merge or production deploy.
