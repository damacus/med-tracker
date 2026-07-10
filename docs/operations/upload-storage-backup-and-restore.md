# Upload Storage Backup and Restore

Production uploads use a deliberately single-node persistent disk mounted at
`/app/storage`. Rails refuses to boot when this path is not a real mount point.
The hosted Kubernetes contract is a Longhorn `ReadWriteOnce` claim with a
`Recreate` deployment. Do not increase web replicas under this contract.

## Recovery Set

Every recoverable backup must contain both parts from the same recovery point:

1. PostgreSQL, including `active_storage_attachments`,
   `active_storage_blobs`, and `active_storage_variant_records`.
2. Every object below `/app/storage`.

The self-service MedTracker ZIP export is not an upload backup and does not
replace this recovery set.

Pause writes or use storage and database snapshots whose recovery timestamps
are coordinated. Encrypt backups in transit and at rest, keep a copy outside the
application cluster, and restrict access to the same operators who can restore
production health data.

The minimum retention contract is 35 daily backups and 12 monthly backups for
both parts of the recovery set. Provider snapshot retention may exceed this,
but shorter retention requires a documented risk decision and data-retention
review.

## Backup Checklist

1. Record the app image tag, schema migration version, database recovery point,
   persistent-volume snapshot identifier, timestamp, and operator.
2. Create the PostgreSQL backup and Longhorn volume snapshot at a coordinated
   recovery point.
3. Copy or replicate both encrypted backups off cluster.
4. Confirm both artifacts are readable and covered by the same retention label.
5. Schedule the isolated restore test; a snapshot is not verified until it has
   been restored and checked.

## Isolated Restore

1. Create an isolated database and persistent volume with no production routes.
2. Restore PostgreSQL and `/app/storage` from the same recorded recovery set.
3. Mount the restored volume at `/app/storage`.
4. Run migrations with the owner-capable database role.
5. Start the app with `ACTIVE_STORAGE_SERVICE=persistent` and
   `ACTIVE_STORAGE_ROOT=/app/storage`.
6. Choose a restored attachment id and run:

```fish
task prod:verify-storage-restore ATTACHMENT_ID=123
```

The check fails if the database attachment is missing, its object key does not
exist on the restored volume, or Active Storage detects a checksum mismatch.
The command prints only attachment id, blob id, and byte size; it does not print
filenames or upload contents.

7. Verify the attachment through its authorized household route and confirm a
   user outside that household cannot retrieve it.
8. Record the recovery point, attachment id, image tag, duration, tester, and
   result. Destroy the isolated environment after evidence is retained.

## Migration and Scaling

Before horizontal scaling, choose an RWX volume with tested multi-node behavior
or migrate blobs to object storage. For object storage, configure provider
credentials outside Git, apply bucket encryption, versioning, lifecycle, and
access policies, copy every existing key, verify database checksums against the
destination, and only then change the selected service. Keep the source volume
read-only until the rollback window expires.
