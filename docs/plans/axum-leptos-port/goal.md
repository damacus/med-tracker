# Cutover-readiness goal

Updated by user direction on 25 September 2026.

Build and verify a cutover-ready Axum and Leptos replacement for MedTracker,
using SeaORM and PostgreSQL 18. Deliver complete usable workflows, beginning
with sign-in, household selection, medication viewing, dose recording, and
updated stock and history. Preserve the shared API for web, Android and iOS.

Use existing Rails tests as the behaviour source. For each bounded workflow,
write and demonstrate failing API/browser tests before implementation, then
make the Rust application pass. Run separate test and product writers with
disjoint ownership, plus independent execution and review. Keep the full
parity inventory as a cutover checklist. Confirmed Rails defects become tests
for intended behaviour; do not preserve defects as requirements.

## Remaining delivery milestones

1. Complete the authenticated medication/dose/stock/history journey.
2. Complete medication management, people/carer permissions, schedules,
   assignments, stock operations and history workflows.
3. Complete responsive, accessible Leptos workflows and PWA installation,
   offline reads, queued doses, reconnect/recovery and duplicate-safe replay.
4. Complete remaining shared API, authentication/OAuth, administration,
   reports, uploads, notifications, FHIR and MCP behaviour.
5. Prove preservation of accounts, medical records and audit history through
   disposable migration and rollback rehearsals; reauthentication is allowed.
6. Measure combined application processes below 200 MB at warm idle and under
   representative workload. Exclude PostgreSQL/supporting services. Compare
   CPU and latency against equivalent Rails workloads without assuming a
   0.1% CPU result.
7. Pass applicable full suites and independent reviews, resolve remaining
   discrepancies, reconcile the authoritative OpenAPI contract and pinned
   client schemas with intended Rust corrections, and produce a
   cutover/rollback runbook.

The goal ends at cutover readiness. Production migration, deployment, merge,
rollout and Rails retirement require separate authorization.

## Execution state

The user authorized continued work. The app-level cutover-readiness goal is
active, verified through the goal tool after the secure-entry checkpoint.
The full objective remains open; completing a bounded workflow does not
complete the cutover goal.

Accepted checkpoints now include secure entry, direct dose recording with
atomic stock updates and replay protection (`39904af0`), and standalone web
sessions using the shared API (`d86c613a`). The session checkpoint passed
eight focused HTTP cases, 36 existing HTTP cases and seven login browser
cases, with independent review.

Shared person, schedule and assignment reads and cookie household discovery
now pass 51 consolidated Rust HTTP cases with independent review. Rust retains
rejected-request audit events that Rails loses through transaction rollback;
the two known Rails failures remain documented.

The next bounded delivery is the complete medication browser journey. Its
actual Rust failure was recorded before UI implementation: four desktop/mobile
cases could not find the medication pages, while the negative-access case
passed. UI implementation and form security/replay tests are in progress.
Known Rails Escape-focus failure remains a required Rust fix. Shared API
record permission and eligible-stock fields will close the remaining picker
permission and alternate-stock gaps without duplicating rules in Leptos.

Full authentication, management, browser/PWA and platform parity, migrated
audit ledger proof and representative performance evidence remain incomplete.
See [ledger.md](ledger.md) for current acceptance evidence.
