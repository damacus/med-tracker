# Durable jobs and recurring schedules Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Preserve every background operation with durable delivery and correct permissions after restart.
**Architecture:** Use Loco 1.2 PostgreSQL BackgroundQueue first. Shared operations own side effects;
workers revalidate actors and use durable idempotency. Verify actual pinned queue capabilities before adapting gaps.
**Tech Stack:** Loco workers/BackgroundQueue, PostgreSQL 18, maintained mail/push clients and timezone support.
**Spec:** [implementation-spec.md](../implementation-spec.md), Rails job inventory and `rails/config/recurring.yml`.

## Global constraints

W1 depends on P3 and runs before enqueueing API endpoints. W2 depends on the relevant accepted operations.
No runtime role DDL, owner bypass or secret-bearing payloads. One queue unless a verified limitation requires
an explicit architecture decision. Worker/source ownership remains with the persistent writer.

## Review focus

Crash after side effect before acknowledgement (W1/W2), duplicate enqueue (W1/W2), withdrawn access
before delivery (W2), daylight-saving boundaries (W3), and SIGTERM with in-flight work (W3) need tests.

### W1: Prove queue provisioning, enqueue and recovery

**Files:** Create `src/workers.rs`, `src/workers/probe.rs`, `tests/queue_runtime.rs`;
modify `src/app.rs`, config worker sections and the P3 queue-storage migration if needed.
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

Each named worker is a separate task with the cycle below, filtered by its module name
in `worker_jobs`. Complete and review one worker before moving to the next. Separate
`reminders::schedule` from `reminders::deliver` within its module and test both filters.
The inventory comparison is the final family gate, not the first deliverable.

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
