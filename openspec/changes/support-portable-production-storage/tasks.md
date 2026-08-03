This change ends when both production backends, both migration directions, backend-aware recovery, and one canary Disk-to-S3-to-Disk round trip are proven. Migrating production, retiring its volume, or splitting web and worker deployments requires a separately approved deployment change.

## 1. Explicit Production Storage Modes

- [x] 1.1 Add failing `ProductionStorage` specs for `persistent`, `persistent_with_s3_mirror`, `s3_with_persistent_mirror`, and `s3`, covering preservation of the existing Disk service identity, backend-specific validation, no undeclared fallback, secret-safe errors, Disk operation without S3 settings, S3 operation without a mount, and unchanged development/test isolation; verify with `task test TEST_FILE=spec/lib/production_storage_spec.rb`.
- [x] 1.2 Implement the four explicit Active Storage services and phase-aware production resolver, retaining fail-closed durable-mount checks only for Disk-inclusive modes and requiring external S3 configuration only for S3-inclusive modes.
- [x] 1.3 Add failing privacy-contract specs and then emit aggregate storage-mode and migration events that exclude credentials, filenames, checksums, contents, household ids, person ids, and medication data.
- [x] 1.4 Add task-wrapped final-production-image smokes that prove Disk-only boot and storage operations without S3 settings and S3-only boot and storage operations without `/app/storage`.

## 2. Symmetric Migration and Cutover

- [x] 2.1 Add failing service specs for Disk-to-S3 and S3-to-Disk backfill, bounded batching, existing valid destinations, missing or corrupt blobs, interruption resumption, and idempotent retries.
- [x] 2.2 Implement one direction-neutral migration service that copies live blobs by logical key, verifies the destination, and reports opaque run ids plus aggregate progress without changing blob service identities.
- [x] 2.3 Add failing specs for both directions covering stable-set reconciliation, mirror-queue drain requirements, cutover blocking, atomic `service_name` transitions, transaction rollback, in-window rollback, finalization, and source-retirement eligibility.
- [x] 2.4 Implement dry-run-by-default reconciliation, cutover, rollback, finalization, and retirement-eligibility services using the symmetric four-state machine and one transaction for each service-name transition.
- [x] 2.5 Expose task-wrapped operator commands for migration start/resume, reconciliation, cutover eligibility, cutover, rollback, finalization, and retirement eligibility; cover source/destination validation, owner-role enforcement, confirmations, exit statuses, and JSON output in command specs.

## 3. Service-Neutral Queued Inputs

- [x] 3.1 Add a migration and failing model specs for NHS dm+d archive service name, opaque logical key, checksum, and byte size while retaining bounded compatibility with existing `archive_path` records.
- [x] 3.2 Add failing archive-store and integration specs for verified persistence before enqueue, resolution through Disk and S3 services, retry-safe reads, terminal cleanup, private access, legacy conversion, and PHI-safe failure behavior.
- [x] 3.3 Implement the archive-store boundary, update the authorized request and `NhsDmdImportJob` to exchange durable service references, and add a bounded command that converts or reports every non-terminal legacy path before its filesystem can be removed.

## 4. Backend-Aware Recovery and Documentation

- [x] 4.1 Add failing restore-verifier specs for Disk-only, S3-only, and mirror-phase recovery sets, including missing or corrupt blobs, owner-role selection, authorized retrieval, cross-household denial, and PHI-safe evidence.
- [x] 4.2 Make restore verification service-agnostic and update the task-wrapped recovery command to record the database plus only the storage recovery references required by live blob service identities and the active migration phase.
- [x] 4.3 Amend ADR 0008 and the backup/restore runbook with both supported steady modes, both migration directions, topology constraints, rollback-versus-future-migration semantics, existing retention objectives, and explicit optional source retirement; validate with `task docs:build`.
- [x] 4.4 Publish a bounded `home-ops` handoff for optional Disk and S3 deployments, canary/production isolation, least-privilege S3 inputs when selected, mount requirements when Disk is selected, migration phase values, maintenance gates, rollback commands, and the prohibition on automatic source deletion.

## 5. Repository Verification

- [x] 5.1 Run focused specs for production storage, both migration directions, operator commands, Active Storage authorization, household export expiry, NHS dm+d requests/jobs, restore verification, and observability; fix every failure before broad gates.
- [x] 5.2 Run `task rubocop`, `task test`, both final-production-image storage smokes, `task docs:build`, `task openspec:validate`, and `git diff --check`; do not publish implementation while any gate is red.

## 6. Bounded Deployed Acceptance

- [ ] 6.1 In isolated canary storage, execute and retain PHI-safe evidence for Disk → Disk-primary mirror → S3-primary mirror → S3, including one verified in-window rollback before S3 finalization.
- [ ] 6.2 From the S3 steady state, provision a fresh canary Disk destination and execute S3 → S3-primary mirror → Disk-primary mirror → Disk, including one verified rollback and final isolated restore.
- [ ] 6.3 Prove upload, authorized download, cross-household denial, analysis, purge, export expiry, NHS dm+d import, background execution, and Loki/Tempo evidence in both final canary modes; record the round-trip result without migrating production or deleting either production storage backend.
