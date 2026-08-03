# Home-Ops Portable Storage Handoff

This handoff is bounded to deployment inputs and maintenance gates for the
portable-storage application contract. It does not authorize a production
migration, volume removal, web/worker split, or source deletion.

## Supported deployment choices

### Disk-only deployment

Set `ACTIVE_STORAGE_SERVICE=persistent`. Mount a writable durable volume at
`ACTIVE_STORAGE_ROOT` in every process that performs storage work. The existing
Longhorn `ReadWriteOnce`, one-pod, `Recreate` topology remains supported. Do not
add S3 settings.

### S3-only deployment

Set `ACTIVE_STORAGE_SERVICE=s3`. Do not mount `/app/storage` in the application
or worker. Supply the endpoint, bucket, region, access key id, secret access
key, and path-style setting through the existing secret-delivery mechanism.
Grant least privilege for object read, write, delete, list, and multipart
operations in the isolated MedTracker bucket; do not grant cluster-wide object
administration.

Canary and production MUST use separate buckets, credentials, Disk volumes,
backup policies, migration runs, and recovery evidence. A canary round trip is
not authority to alter production.

## Migration phases

The deployment value must match the operator workflow exactly:

- `persistent`: Disk steady state.
- `persistent_with_s3_mirror`: Disk primary, S3 mirror.
- `s3_with_persistent_mirror`: S3 primary, Disk mirror.
- `s3`: S3 steady state.

Disk to S3 advances through the list; S3 to Disk moves through it in reverse.
Keep both storage systems available throughout either mirror phase and its
rollback window.

## Maintenance gates

Before cutover, put the application in a maintenance state that stops attachment
mutations and storage-dependent job dispatch. Drain mirror, analysis, purge,
export-expiry, and NHS dm+d work. Then run reconciliation, cutover eligibility,
and the dry-run cutover command. Apply only with the direction-specific
confirmation value.

The application tasks are:

```fish
task prod:storage-migration-start
task prod:storage-migration-resume
task prod:storage-migration-reconcile
task prod:storage-migration-cutover-eligibility
task prod:storage-migration-cutover
task prod:storage-migration-rollback
task prod:storage-migration-finalize
task prod:storage-migration-retirement-eligibility
```

The task input includes source, destination, current phase, run id where
applicable, explicit gate values, `APPLY=true`, and `CONFIRM=persistent-to-s3`
or `CONFIRM=s3-to-persistent`. Omitting `APPLY=true` is a dry run.

## Rollback and retirement

Before the deadline, verify the source and drain mirror work, then use the
rollback task and return the deployment to the source-primary mirror phase.
After finalization, returning to the former backend is a new migration, not
rollback.

The application reports retirement eligibility only. Home-Ops automation MUST NOT delete
a bucket, credential, PersistentVolumeClaim, snapshot, backup, or
recovery history automatically. Source retirement requires a separate approved
deployment change and explicit operator sign-off.
