## 1. Incident, demo mode, and baseline contract

- [x] 1.1 File a GitHub issue for the production-data-in-canary incident and replace the proposal's no-issue note with the issue link.
- [x] 1.2 Run `task test:preflight` and record the current focused seed/reset test baseline before implementation.
- [x] 1.3 Write failing RSpec and system coverage proving demo mode defaults off, is enabled only by explicit configuration, shows an accessible disposable-data and weekly-reset notice on authenticated pages, and changes no authorization or medication behavior.
- [x] 1.4 Implement the application-owned demo-mode boundary and shared-layout notice, then make the focused configuration and system specs pass.
- [x] 1.5 Write failing RSpec coverage for loading the allow-listed synthetic baseline into an empty migrated database, including representative roles, scheduled and PRN medication scenarios, current-date takes, tenant/grant integrity, documented demo authentication, and zero baseline notification destinations.
- [x] 1.6 Add the dedicated `db/demo/` dataset and demo loader by promoting only the required synthetic fixture scenarios, then make the focused baseline specs pass.

## 2. Guarded reset behavior

- [x] 2.1 Write failing RSpec coverage proving reset refuses disabled demo mode or mismatched application host, database host, configured storage target, and owner-capable connection without mutating data or logging sensitive values.
- [x] 2.2 Implement the canary-only reset preflight and PHI-safe stage/result reporting, then make the safety specs pass.
- [x] 2.3 Write failing RSpec coverage for transactional runtime-table replacement, schema metadata preservation, rollback on baseline-load failure, removal of sessions/jobs/cache/audit/subscription/token state, and idempotent repeated execution.
- [x] 2.4 Implement advisory locking plus transactional table replacement and baseline loading, then make the database reset specs pass.
- [x] 2.5 Write failing RSpec coverage for post-commit upload cleanup, safe retry after cleanup failure, empty-root verification, and failure when the storage target escapes the verified canary mount.
- [x] 2.6 Implement verified canary storage cleanup and post-reset invariant checks, then make the storage/reset specs pass.
- [x] 2.7 Add the operator Rake/Task entry point and a focused command contract spec proving it returns non-zero for safety, reset, cleanup, or verification failures.
- [x] 2.8 Detect the configured Active Storage service, require its exact disk and/or S3 targets, and clean and verify every backend used by disk, S3, or mirror services.
- [x] 2.9 Move the real `/up` verification out of the Rails command so it runs only after the Kubernetes wrapper restores the web deployment.

## 3. MedTracker verification and image publication

- [x] 3.1 Run focused seed/reset specs through `task test TEST_FILE=...`, then run `task rubocop` and `task test` with all application changes green.
- [x] 3.2 Run the application in demo mode and capture mobile and desktop screenshots proving the notice remains visible and accessible on the real authenticated UI.
- [x] 3.3 Run `task openspec:validate` and `git diff --check`, review the diff for production behavior changes, and confirm demo mode remains off by default while notification delivery remains available to test users.
- [ ] 3.4 Commit and push the MedTracker change, open its scoped PR, keep checks green, and publish or identify the exact reset-capable image SHA for canary.

## 4. Home-ops canary reset controller

- [ ] 4.1 Create a fresh writable `home-ops` worktree from current `origin/main` and confirm the live canary image, database, PVC, backup, ObjectStore, and RustFS bucket identities before editing.
- [ ] 4.2 Add failing assertions to `task kubernetes:med-tracker-canary-isolation` for `DEMO_MODE=true` only in canary, a same-image CronJob controller, Sunday 04:15 Europe/London schedule, `Forbid` concurrency, exact canary database and S3 targeting, no application PVC, an exclusive mutation-writer window, real application health verification after restart, and absence of production recovery or every canary database backup/archive resource.
- [ ] 4.3 Add the reset CronJob controller to the canary HelmRelease using the exact MedTracker image SHA and demo-mode configuration, owner-capable canary database role, isolated canary S3 bucket, bounded runtime/retries/history, no Puma or Solid Queue process in the reset runner, no application storage mount, and orchestration that quiesces user mutations and queue writers until strict reset verification succeeds.
- [ ] 4.4 Reduce canary CNPG to one clean `initdb` instance with ordinary storage and monitoring, disable superuser access, and remove `externalClusters`, every physical or logical backup resource, the Barman WAL-archiver plugin, ScheduledBackup, ObjectStore, and canary RustFS backup credential/bootstrap resources.
- [ ] 4.5 Remove the misleading `PUSH_NOTIFICATIONS_ENABLED=false` canary value while retaining normal test-user notification delivery and existing edge access protection.
- [ ] 4.6 Update the canary README with application demo mode, scheduled/manual reset operation, disposable-data warning, safety prerequisites, application-baseline recovery, first-run procedure, failure recovery, and notification-testing behavior.
- [ ] 4.7 Provision and validate a least-privilege canary upload bucket identity, make the focused isolation test green, render the canary app/database and affected parent kustomizations, run `git diff --check`, and complete the repo's required narrow/full validation.
- [ ] 4.8 Commit and push the home-ops change, open its scoped PR, and keep checks green without reconciling destructive resources yet.

## 5. One-time production-data removal

- [x] 5.1 Capture read-only evidence of the exact live canary deployment, CNPG cluster, database PVCs, application-storage PVC, Backup objects, ObjectStore destination, and `s3://cnpg-med-tracker-canary/` target, then re-confirm approval.
- [x] 5.2 Stop canary web and job execution, suspend both Flux Kustomizations through GitOps, and verify the exact canary-only deletion scope before deleting data.
- [x] 5.3 Delete only CNPG cluster `home/med-tracker-canary`, PVCs `med-tracker-canary-1` and `med-tracker-canary-2`, PVC `med-tracker-canary-storage`, canary Backup objects, and objects under `s3://cnpg-med-tracker-canary/`; verify production cluster, PVCs, backups, and bucket remain unchanged.
- [ ] 5.4 Reconcile a fresh one-instance canary database and empty upload bucket, let migrations complete, manually trigger the application reset from the CronJob, and require every post-reset invariant to pass before starting canary traffic.

## 6. Operational acceptance

- [ ] 6.1 Sign in with a documented demo user and verify the demo notice and reset schedule, household-role access, scheduled medication, PRN medication, current-date history, and absence of production-derived records.
- [ ] 6.2 Register a test device, verify one canary notification is delivered, rerun the reset manually, and verify the registration and other test-created data are removed.
- [ ] 6.3 Delete and recreate canary database state in a non-production acceptance run, then verify migrations plus the application baseline restore the demo without any database Backup, dump, WAL archive, ObjectStore, production-recovery reference, or retained upload.
- [ ] 6.4 Observe the first Sunday 04:15 Europe/London execution, confirm exactly one successful non-overlapping job and healthy canary afterward, and attach PHI-safe evidence to the incident issue.
