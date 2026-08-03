## Why

The canary deployment was restored from a production backup and therefore inherited real household data and live notification subscriptions. Canary is now a demo environment, so it needs a reproducible synthetic baseline and an automatic weekly reset that removes changes made by test users without affecting production.

Originating issue: [damacus/med-tracker#1780](https://github.com/damacus/med-tracker/issues/1780). The affected deployment originated in [damacus/home-ops#3783](https://github.com/damacus/home-ops/pull/3783).

## What Changes

- Add an explicit application-owned demo mode, disabled by default, that identifies canary as disposable and shows users when its data resets.
- Add a demo-mode reset operation that removes all runtime data and uploaded files, then loads a deterministic synthetic baseline derived from an allow-listed subset of the existing fixture scenarios.
- Add a weekly Kubernetes CronJob that invokes the reset outside Solid Queue, prevents overlapping runs, and verifies the restored baseline.
- Require demo mode plus exact canary target assertions before destructive reset work can begin.
- Make the versioned application dataset the sole canary recovery baseline: a blank database is migrated and populated by the deployed application rather than restored from a database backup.
- Simplify canary CNPG to a single clean-initialized instance with no production recovery source, physical or logical baseline backup, scheduled backup, object store, or WAL archiving.
- Keep notifications available to test users; subscriptions and device tokens created during testing are ordinary demo data and are removed by the next reset.
- Document and verify the one-time transition that destroys the current production-derived canary database, application storage, and canary-only backup objects before recreating the environment from synthetic data.
- **BREAKING**: Canary data is disposable. Any account, medication, take, subscription, token, audit record, job, cache entry, or upload created outside the committed baseline is removed by the weekly reset.

Non-goals:

- Changing production database, backup, retention, or notification behavior.
- Disabling canary notifications for intentionally registered test users.
- Restoring canary from production data for migration rehearsal.
- Preserving user-created canary data across weekly resets.

## Capabilities

### New Capabilities

- `canary-demo-reset`: Defines application demo mode, the synthetic canary baseline, destructive safety boundary, weekly reset behavior, post-reset verification, and production-data-free bootstrap contract.

### Modified Capabilities

None.

## Impact

- MedTracker demo-mode configuration and presentation, seed/reset services, fixture-derived demo data, Rake/Task entry points, Active Storage cleanup, and RSpec/system coverage.
- The `med-tracker-canary` application and `med-tracker-canary-db` manifests in `damacus/home-ops`, including a minimal CNPG cluster, persistent storage, and a Kubernetes CronJob.
- Canary operations: the production-derived CNPG volumes, Longhorn application storage, Backup resources, and `s3://cnpg-med-tracker-canary/` were destroyed on 2026-08-03 after Flux reconciliation was suspended. Canary remains suspended until the replacement contracts are deployed.
- No public API contract changes and no production runtime behavior changes.
