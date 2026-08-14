# Upload Storage Backup and Restore

Production supports durable Disk and S3-compatible storage. The selected
backend and any active migration determine the recovery set. S3 is optional.

## Recovery set

Every recovery point includes PostgreSQL, including
`active_storage_attachments`, `active_storage_blobs`,
`active_storage_variant_records`, and `storage_migration_runs`.
Add storage references as follows:

- **Disk-only**: a coordinated snapshot or backup of `/app/storage`.
- **S3-only**: a protected bucket recovery point; no blob-volume snapshot is
  required.
- **Mirror or rollback-window**: coordinated Disk and S3 recovery points.

The database and required storage references must describe the same recovery
point. A self-service MedTracker ZIP export does not replace this set.

Encrypt backups in transit and at rest, retain an off-cluster copy, and restrict
access to recovery operators. Keep at least 35 daily backups and 12 monthly backups
for every required part. Shorter retention requires a documented risk
and data-retention decision.

## Backup checklist

1. Quiesce storage mutations or use coordinated point-in-time mechanisms.
2. Record the app image, schema version, timestamp, operator, database recovery
   reference, selected storage service, and active migration run and phase.
3. Record `DISK_RECOVERY_REFERENCE` only when Disk is required.
4. Record `S3_RECOVERY_REFERENCE` only when S3 is required.
5. Confirm every required artifact is readable and has the same retention
   label.
6. Schedule an isolated restore. A backup is not accepted until that restore
   passes.

## Isolated restore

The `task prod:verify-storage-restore` command starts a local Docker Compose
production service. Use it to check a local restored environment. It does not
select a hosted restore target.

For a deployed isolated restore, run `rails med_tracker:storage:verify_restore`
inside the restored application Job or pod. Set the same evidence variables and
confirm the target database and storage service first.

1. Create an isolated database and storage destination with no production
   routes.
2. Restore PostgreSQL and the required Disk, S3, or dual storage set.
3. Configure the restored service identity exactly as recorded. Mount
   `/app/storage` only for a Disk-inclusive phase.
4. Run migrations with `DATABASE_ROLE=med_tracker_owner`.
5. Start the application and choose a restored attachment id.
6. Verify authorized retrieval through the application and denial from a user
   in another household.
7. Run the backend-aware verifier. This Disk example intentionally omits an S3
   reference:

```fish
task prod:verify-storage-restore \
  ATTACHMENT_ID=123 \
  DATABASE_RECOVERY_REFERENCE=db-snapshot-opaque \
  DISK_RECOVERY_REFERENCE=disk-snapshot-opaque \
  AUTHORIZED_RETRIEVAL_VERIFIED=true \
  CROSS_HOUSEHOLD_DENIAL_VERIFIED=true
```

For S3-only recovery, provide `S3_RECOVERY_REFERENCE` instead. During a mirror
or rollback phase, provide both and include `STORAGE_MIGRATION_RUN_ID`.

The command fails for an absent record, unavailable service, missing object,
checksum mismatch, incomplete access evidence, wrong database role, or missing
required recovery reference. Its JSON evidence excludes filenames, contents,
checksums, household ids, person ids, and medication data.

8. Record duration, tester, command result, and deployment-level recovery
   evidence. Destroy the isolated environment after evidence is retained.

## Migration recovery and source retention

Protect both storage backends from the first mirror phase until finalization.
An in-window rollback uses the recorded migration run and returns to the
source-primary mirror service. After finalization, moving back is a future
reverse migration with full backfill and verification.

Keeping the inactive source is valid. The application never removes it.
Retirement requires an expired rollback window, acceptance evidence, an
isolated recovery proof, final reconciliation, no live dependency, and explicit
deployment approval.
