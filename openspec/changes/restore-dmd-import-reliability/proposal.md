## Why

Production DM+D imports stopped showing live progress and could be terminated by the shared web and job-process memory limit, leaving imports permanently active. The application repair is now merged in [damacus/med-tracker#1784](https://github.com/damacus/med-tracker/pull/1784), but its deployment work overlaps the in-flight canary demo-reset stream and the remaining portable-storage canary evidence, so the intended outcome and merge order need one explicit contract before either stream is reshaped.

No dedicated incident issue was filed; the production evidence and implemented application boundary are recorded in PR #1784. Related deployment-dependent canary evidence, including DM+D and background-job behavior, remains tracked in [damacus/med-tracker#1775](https://github.com/damacus/med-tracker/issues/1775). The related canary reset work originates from [damacus/med-tracker#1780](https://github.com/damacus/med-tracker/issues/1780).

## What Changes

- Preserve real-time DM+D import progress through a signed Turbo Stream subscription and committed refresh broadcasts, with no inline reload script.
- Bound importer memory, enforce one active import, and automatically fail stale interrupted imports while preserving existing upload, progress-counter, and final-result behavior.
- Require production and canary job execution to run outside Puma in a dedicated worker with the same release configuration and shared application storage, plus an independent memory limit.
- Coordinate the DM+D rollout with the weekly canary demo-reset work so the canary can validate a full public release before the identical worker topology is promoted to production.
- Treat merged portable storage ([damacus/med-tracker#1782](https://github.com/damacus/med-tracker/pull/1782)) and merged DM+D reliability (#1784) as historical prerequisites, retain [issue #1775](https://github.com/damacus/med-tracker/issues/1775) as the canary storage and restore-evidence gate, and preserve the unresolved medication-administration persistence observation as a PHI-safe operational evidence lane.
- Keep the boundary between backup-free canary database recovery and optional application blob storage explicit: removing database backup/archive resources MUST NOT be interpreted as disabling durable application archives or S3-compatible application storage.
- Keep this OpenSpec change rooted in MedTracker because it defines delivery of a MedTracker production capability. Its authorized implementation scope crosses into `damacus/home-ops`, whose deployment configuration is production code required to ship and operate the application.
- Keep code ownership explicit inside that single MedTracker-owned plan: MedTracker owns application behavior and tests; `damacus/home-ops` owns deployment topology and release activation. MedTracker artifacts may reference the deployment repository, PRs, and behavioral contract but MUST NOT encode home-ops filesystem or manifest paths.
- Deliver the overlap as repository-local stacked work: the merged MedTracker DM+D contract precedes the canary reset integration, while [damacus/home-ops#3995](https://github.com/damacus/home-ops/pull/3995) is the planned lower deployment boundary and [damacus/home-ops#3999](https://github.com/damacus/home-ops/pull/3999) is planned to be rebuilt above it from its current `main` base.
- Execute the remaining plan through agent-orchestrated development: this coordinating agent owns architecture, stack lineage, integration, and final verification; fresh bounded subagents implement and independently review each task under adaptive model routing.

Non-goals:

- Creating a cross-repository Git branch stack.
- Moving the planning authority or OpenSpec source of truth out of MedTracker.
- Moving deployment manifests, topology assertions, or deployment paths into MedTracker.
- Changing DM+D routes, archive format, counters, or completed import results.
- Reconciling canary or promoting production before both repository stacks are green and a full canary import succeeds.

## Capabilities

### New Capabilities

- `dmd-import-reliability`: Defines observable live progress, bounded and exclusive import execution, interrupted-run reconciliation, deployment-worker isolation, and coordinated canary acceptance.

### Modified Capabilities

None.

## Impact

- MedTracker DM+D import presentation, persistence lifecycle, archive importer, background jobs, recurring schedule, database invariant, and automated coverage.
- The `damacus/home-ops` production code for the canary and production worker topology represented by PR #3999, planned to be restacked above the demo-reset deployment contract in PR #3995.
- Release sequencing: the application image is available before deployment activation; canary proves progress, completion, counts, logs, memory bounds, and web stability before production promotion.
- Release evidence: portable storage, DM+D reliability, canary reset, worker isolation, and the unresolved reminder-persistence observation each retain an explicit owner, current state, and acceptance disposition even when they do not belong in the same Git branch stack.
- Delivery process: a durable progress ledger, task-specific agent briefs, independent task review, and final whole-branch review prevent stack or concurrency decisions from being delegated without parent judgment.
- Existing HTTP interfaces, upload format, import counters, final results, Solid Cable transport, and household or medication data contracts remain unchanged.
