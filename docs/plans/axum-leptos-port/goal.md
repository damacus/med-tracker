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

The first medication browser journey now passes all seven selected cases:
desktop/mobile recording, updated stock/history, Escape focus restoration,
access denial, form rejection/replay and effective taper dosing. Leptos uses
the shared API for these reads and writes. API-owned recording permissions
and eligible-stock choices avoid duplicating medication rules in the web UI.
The original missing-page failures, URL/source mismatch and taper regression
were demonstrated against earlier Rust snapshots before final acceptance.

The current delivery is medication management: create/edit medication,
stock operations and permission checks, with the full onboarding wizard,
edit and refill forms developed in parallel. The shared Rails browser
baseline passes both management journeys. Rust onboarding and supporting
read tests demonstrate missing routes and capability fields (eight and
three failing cases respectively); implementation remains outstanding.

Medication management now passes 72 consolidated API cases, seven login
browser cases and seven existing medication browser cases. Medication and
tracked stock changes emit attributed sync events, including person metadata
for recorded doses. Replays and rejected writes do not emit duplicate domain
events in the covered cases. Next complete atomic wizard
creation and refill through the shared API and run the same browser journeys
against Rust. Keep test, API and UI writers on disjoint paths while immutable
Compose snapshots provide independent execution and review.
An exact effective-dose preview and clearer alternate-stock confirmation
remain UI parity gaps; the first journey does not establish full web parity.

Full authentication, management, browser/PWA and platform parity, migrated
audit ledger proof and representative performance evidence remain incomplete.
See [ledger.md](ledger.md) for current acceptance evidence.
