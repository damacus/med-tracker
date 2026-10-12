# Durable jobs and recurring schedules Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Preserve every background operation with durable delivery and correct permissions after restart.
**Architecture:** Use Loco 1.2 PostgreSQL BackgroundQueue first. Shared operations own side effects;
workers revalidate actors and use durable idempotency. Verify actual pinned queue capabilities before adapting gaps.
**Tech Stack:** Loco workers/BackgroundQueue, PostgreSQL 18, maintained mail/push clients and timezone support.
**Spec:** [implementation-spec.md](../implementation-spec.md), Rails job inventory and `rails/config/recurring.yml`.

## Global constraints

Loss of pending/failed Rails jobs at cutover is explicitly accepted. Do not transfer
the old queue or reconcile individual historical deliveries. All Loco job, permission,
retry, duplicate-suppression, recovery and schedule checks remain required.
Automated NHS dm+d import/reconciliation and medication review generation/background
refresh are explicitly required before first production. Optional AI/provider
lookup deferral cannot defer these outcomes; report a genuine required dependency
as unresolved rather than silently omitting the job.

W1 depends on P3 and runs before enqueueing API endpoints. W2 depends on the relevant accepted operations.
No runtime role DDL, owner bypass or secret-bearing payloads. One queue unless a verified limitation requires
an explicit architecture decision. Worker/source ownership remains with the persistent writer.

Browser Web Push is the first notification capability to complete. Reuse the
existing browser opt-in/unsubscribe/test flow and service-worker payload/click
contract. Keep account-scoped, multiple-device subscriptions in PostgreSQL,
alongside durable queue coordination, with the same VAPID signing key across
replicas. RustFS owns shared files rather than subscription or job coordination.
Use a maintained Web Push implementation. The retained Rust input already uses
`web-push` 0.11 with default features disabled and its public request builder,
sending the library's signed/encrypted message through `reqwest` with Rustls.
Reuse that maintained protocol path rather than introducing the optional
isahc/libcurl or Hyper/native-TLS clients. It is not installed in the root app
yet. The encryption dependency still requires inspection and static-link proof;
verify the full dependency chain and HTTPS trust roots in both scratch images.

Preserve HTTPS/endpoint allowlisting/private-address protections and recheck
household access and notification preferences at delivery. Deduplicate logical
reminders in PostgreSQL and prove two workers cannot deliver the same reminder
twice. Honour Retry-After for temporary failures within the reminder's useful
lifetime; remove only the individually expired 404/410 endpoint. Provider
acceptance does not prove that the browser displayed the notification. Shared
browser account switching must not deliver the previous account's health data.
Verify actual Chrome/Firefox/Safari and iPhone Home Screen delivery, click routing,
opt-out, expiry/retry and release-image HTTPS. Provide feature detection and iOS
installation guidance, and request permission only after a deliberate user action.

## Review focus

Crash after side effect before acknowledgement (W1/W2), duplicate enqueue (W1/W2), withdrawn access
before delivery (W2), daylight-saving boundaries (W3), and SIGTERM with in-flight work (W3) need tests.

### W1: Prove queue provisioning, enqueue and recovery

**Files:** Create `src/workers.rs`, the first concrete W2 worker below and `tests/queue_runtime.rs`;
modify `src/app.rs`, config worker sections and the P3 queue-storage migration if needed.
Prove queue delivery with that real job's observable outcome. Do not add a
temporary probe worker, parallel queue adapter or extra runtime runner.
**Interfaces:** Standard Loco worker registration in `App::connect_workers`; new
`enqueue(ctx: &AppContext, input: JobInput) -> Result<(), OperationError>` in each concrete worker.
`JobInput` contains `actor_id: i64`, `household_id: i64`, `operation_key: String` and the specific job payload.

- [ ] Test enqueue then process restart, forced failure/retry, abandoned-job recovery and duplicate delivery.
  Assert `job_survives_restart == true`, `side_effect_count == 1`, `runtime_ddl_permitted == false`.
- [ ] Run `rtk task slice:test TARGET=queue_runtime`; record missing registration/recovery guarantees.
- [ ] Use the pinned queue's documented capabilities, with storage preprovisioned by P3.
  Add only demonstrated necessary framework adapters; document recovery/retry semantics and observability.
- [ ] Run queue tests with real worker processes and least-privilege PostgreSQL 18; root CI must pass.
- [ ] Review and commit `feat(workers): establish durable PostgreSQL job delivery`.

### W2: Migrate all inventoried jobs by observable outcome

**Files:** Create `src/workers/{dmd_import,dmd_reconciliation,reminders,missed_doses,low_stock,
review_refresh,export_expiry,support_expiry,observability_canary}.rs`,
`tests/worker_jobs.rs`; register them in `src/app.rs`.
**Interfaces:** Each named worker's `enqueue` follows W1 and invokes the accepted domain operation.
Preserve all Rails inventoried job outcomes, including reminder scheduling/delivery as separate responsibilities.

Use the focused cycle below for each named worker, filtered by its module name in
`worker_jobs`. Group related jobs for review and publication: notifications and
reminders; imports and reconciliation; expiry and operational jobs. Resolve each
job's failing checks before proceeding, but do not add a separate review, full CI
run or publication gate for every worker. Review the complete family alongside
focused checks, then run final CI once after review corrections are settled.
Separate `reminders::schedule` from `reminders::deliver` within its module and test
both filters. The inventory comparison remains the final family gate.

- [ ] For every inventory job, test successful delivery, repeated execution, external failure and current
  permission denial. Assert `inventoried_jobs_without_replacement == []`, `repeat_side_effect_count == 1`,
  `revoked_actor_delivery_count == 0`; verify preference checks for all notification categories.
- [ ] Run `rtk task slice:test TARGET=worker_jobs`; record missing concrete jobs.
- [ ] Implement per-job workers with current RLS/actor context, maintained push/mail clients, provenance
  and durable completion/idempotency. Never treat an enqueue response as completed clinical work.
- [ ] Run each job against controlled external failures and process restart; verify delivery/audit/storage state.
- [ ] Review and commit independently testable job families; publish their accepted outcomes in the report.

### W3: Recurrence, graceful drain and operator visibility

**Files:** Create `src/tasks.rs`, `src/tasks/schedules.rs`, `tests/worker_lifecycle.rs`;
modify config schedule/worker sections, `Taskfile.yml` and operating runbook.
**Interfaces:** Standard registered Loco schedules reproduce the inventory; server and worker remain
separate `task dev`/`task worker` processes. Failed/retrying work has observable status without health data in logs.

- [ ] Test daylight-saving gap/fold, overdue recurring work, repeated scheduler tick, SIGTERM mid-job
  and recovery after forced termination. Assert `scheduled_side_effect_count == 1`,
  `unfinished_job_lost == false`, `health_data_logged == false`.
- [ ] Run `rtk task slice:test TARGET=worker_lifecycle`; record actual shutdown/schedule failures.
- [ ] Integrate maintained timezone and queue shutdown behaviour; preserve original recurrence semantics.
  Define readiness, liveness, retry/failure visibility and operator recovery commands.
- [ ] Run the complete worker suite, root CI and controlled restart/drain rehearsals with owned resources.
- [ ] Obtain lifecycle/permission review, integrate and publish; update all inventoried job/schedule evidence.

**Done:** Every job and recurring schedule has delivery, retry, idempotency, permission, restart and
shutdown evidence. Compilation and queue table existence alone do not satisfy this slice.

## Browser notification reliability delivery — 12 October 2026

Spec: the owner-approved notification plan in this session supplements the browser-first requirements above. These Tasks implement the notification subset; other W2 jobs remain separate.

### Global Constraints

- Due and missed alerts expire one hour after intended notification time; missed notification time is the scheduled dose time plus the existing 30-minute grace period. Low-stock expires 24 hours after threshold crossing.
- Uncertain sends are not automatically retried. Each device has independent durable outcomes. Recheck current permissions, preferences, subscription ownership and dose/stock relevance before each attempt.
- Reuse Loco PostgreSQL queue and maintained Web Push. No native APNs/FCM delivery or production deployment. Do not invent cryptography or a second queue.
- Five total attempts per device, delays 1, 5, 15, 30 minutes after successive confirmed temporary failures; Retry-After is a lower bound, never exceed expiry. Ten-second transport timeout; a stranded sending claim becomes uncertain after 120 seconds.
- Preserve public APIs and existing comments. Use TDD and project Tasks; no Rails code changes expected. Keep tenant isolation, audit rollback, privacy and current account checks.
- One implementation writer at a time, fresh isolated-context Sol agents per task, independent task review and a final Astra concurrency/security review subject to usage policy. No nested subagents.
- Refresh PR overlap before integration: #2541 owns proposed subscription lock-order changes; #2505 owns dashboard and shared schedule projection changes; #2540 owns current-profile API; #2528 CI tooling; #2467 dependencies. Do not duplicate their implementation.
- Use only synthetic isolated fixtures. No live credentials, production writes or unsolicited notifications to other people. Physical-device evidence and release-image acceptance must be reported honestly if unavailable.
- Each task implements, runs focused tests, self-reviews and commits explicit files after checking author/committer/signing identity. Coordinator owns full final CI, publication and hosted verification. Unsigned per-command fallback is allowed if 1Password signing is unavailable; never disable signing globally.

### Task 1: Durable per-device delivery, classified failures and bounded recovery

Read Global Constraints above as binding. Inspect current notification_events, push_subscriptions, tenant migration and queue patterns before choosing the smallest compatible implementation.
Add a tenant-protected delivery table with event/subscription uniqueness, account/household ownership, status, attempt count, next-attempt, claim time and bounded safe diagnostic outcome. Preserve historical event rows; previously accepted/failed/unknown events must not be replayed by migration. Snapshot eligible subscriptions when creating deliveries, never add devices during retries. Subscription deletion/reassignment must invalidate pending deliveries and must not erase accepted outcomes needed to avoid replays.
Commit a sending claim and required audit before each external send; release DB locks for network I/O. Guard outcome updates by exact claim/attempt and status. Accepted rows remain terminal; recover sending older than 120 seconds as uncertain and suppress automatic resend. Recovery must not overwrite another terminal result.
Classify Accepted, Expired, Permanent rejection, Confirmed temporary rejection, and Uncertain. Timeouts/ambiguous outcomes are uncertain, not retryable. Remove only individual expired subscriptions. Retry only proven nonacceptance. Five attempts maximum with 1/5/15/30 minute delays after prior confirmed failures and Retry-After lower bound. Deadline always wins. Parse HTTP-date and delta-seconds Retry-After where supported by the maintained transport; inspect pinned library/Context7 before adapting. Do not create bespoke Web Push.
Store intended_at and expires_at for each logical notification; current clock governs every attempted send. Rebuild payload from current preferences, revalidate access/dose/stock/ownership on each attempt; provider TTL must not exceed remaining lifetime. Preserve current five-minute TTL as an upper bound rather than extend provider queue exposure.
Use current shared lock helper without copying #2541 patch; report concrete conflicts. Minimal app/queue registration edits only.
RED/GREEN: two devices partial success, concurrent claims, crash before/after external acceptance, uncertain timeout, 120-second recovery, stale result update, max attempts/backoff/Retry-After/deadline, revoked ownership/access, audit rollback. Existing public test-send behavior remains truthful and compatible.
Report exact interfaces/schema and tested commands for Task 2 to consume. Do not implement scheduler catch-up in this task, but expose a clear operation to drain eligible device retries independently from new-intent discovery.

### Task 2: Scheduler catch-up with current-state eligibility

Read Global Constraints and Task 1 report interfaces. Change exact-minute discovery into bounded lookback: dose_due and missed notification instants within the prior hour, low-stock events within 24 hours, deduplicated by logical event identity. Compute intended notification time from saved account timezone and existing occurrence semantics; keep the missed 30-minute grace. Use present time for expiry/relevance, never the stale queue timestamp.
The minute scheduler and worker must discover newly eligible intents AND drain persisted retries independently; a retry must not depend on an exact-minute intent being rediscovered. Stop obsolete reminders after taken/not-taken outcomes, pauses, permission withdrawal, opt-out or replenishment. Preserve recipient/grant rules and private text.
Use existing occurrence projection and dashboard-owned scheduling helpers; do not duplicate #2505 work. Preserve stable event identities through repeated ticks/restart and handle DST gaps/folds according to existing occurrence semantics.
RED/GREEN tests: missed scheduler ticks catch up within windows, exact expiry boundary (now >= expires_at suppresses), no late sends, stale queued timestamps, restart and concurrent ticks, saved timezones/DST, taken dose/stock refill/opt-out between scheduling and delivery.
Report final integration interfaces and remaining verification work.

### Task 3: Browser and operational acceptance

Read Global Constraints and previous reports. Extend focused browser/worker tests for subscribe/test/unsubscribe, notification payload/click paths, expired endpoint removal, account switching and retained private text, using synthetic fixtures. Inspect and fix in-scope defects through RED/GREEN; preserve actual notification display versus provider acceptance distinction.
Document scheduler/worker setup, per-device accepted/retryable/uncertain/expired outcomes, bounded recovery and safe operational inspection without health/credential logging. Update this worker record with actual evidence, remaining real-device and release-image gates.
Use existing automated browser checks and safe owned local synthetic runtime if available. Do not configure live push credentials or send real notifications without the authorised test device/setup. If real-device evidence is unavailable, record the exact gap; do not simulate it into acceptance.
Coordinator runs final task ci and docs build after all task reviews and final source corrections. No broad CI re-runs per trivial edit. No production deployment or merge.

### Browser notification reliability evidence and remaining gates

- Local synthetic integration proves missed-tick catch-up, stable event identity, expiry, current-state suppression, independent device outcomes, bounded retry, stale-claim recovery and preserved private-text preference. The focused `browser_push` care suite passed 28/28 on 12 October 2026.
- Local worker checks passed 15/15 and prove the service worker verifies the signed-in account before displaying a payload or opening its destination. Account transitions drain in-flight status checks and displays, close retained notifications and suppress display across worker restarts. A different tab cannot release the transition. The service worker suppresses a payload when the account check fails, including while signed out or offline.
- The browser profile flow passed 6/6 on desktop, including recovery-code sign-in cleanup, and the earlier baseline passed 5/5 on mobile Chromium. A separate desktop passkey sign-in journey passed 1/1. These used an owned local database and synthetic VAPID key. The profile flow exercises subscription registration, test-send response, expired status, re-enable and unsubscribe. Provider acceptance remains distinct from actual device display. The operational procedure is in `docs/operations/notification-delivery.md`.
- Coordinator gates remain: final `task ci` and `task docs:build` after review and source freeze, dependency/static-link and HTTPS trust-root proof in release images, physical browser and iPhone Home Screen receipt/click/opt-out tests on an authorised device, and deployment approval. None of the physical-device, release-image or production gates is claimed by this local synthetic evidence.
