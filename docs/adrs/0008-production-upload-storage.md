# ADR 0008: Production Upload Storage

- Status
  Accepted
- Date
  2026-07-10
- Amended
  2026-08-02

## Context

The original decision made production Disk storage fail closed. A real durable
mount at `/app/storage`, a single web replica, a `ReadWriteOnce` claim, and a
`Recreate` rollout prevented uploads from silently landing in an ephemeral
container filesystem. Those constraints remain valid for Disk deployments.

MedTracker must also support customers that select S3-compatible object storage
and must not make S3 a requirement for customers that continue to use Disk.
Operators need a verified migration in either direction without rewriting
logical blob keys, weakening attachment authorization, or assuming that a
retired source is still current.

## Decision

Production explicitly selects one of four Active Storage services:

1. `persistent`: disk-only steady state.
2. `persistent_with_s3_mirror`: disk-readable migration state.
3. `s3_with_persistent_mirror`: S3-readable migration or rollback state.
4. `s3`: S3-only steady state.

Disk-inclusive modes retain the original fail-closed `ACTIVE_STORAGE_ROOT`
checks. S3-inclusive modes require endpoint, bucket, region, access key, and
secret key configuration. `persistent` remains the default, so a customer can
run or upgrade MedTracker without S3 settings.

### Topology constraints

A Disk-only deployment may use the existing `ReadWriteOnce` and `Recreate`
single-pod topology. Every process that performs storage work must see the same
durable root. Horizontal web scaling is not supported by that topology unless
the operator separately proves suitable shared-filesystem semantics.

S3-only deployments do not mount `/app/storage`; independently scheduled web
and worker pods are permitted because the durable object service is shared.
Selecting S3 does not itself require a web/worker split.

### Symmetric migration

Disk to S3 uses `persistent` → `persistent_with_s3_mirror` →
`s3_with_persistent_mirror` → `s3`. S3 to Disk uses the same states in reverse.
Backfill copies by logical key and verifies recorded checksums. It tolerates
valid existing destinations and resumes safely. Cutover requires quiesced storage
mutations, drained mirror work, and a stable fully verified blob set. Blob
service identities change in one database transaction.

During the rollback window, writes continue to both services. An in-window rollback
returns to the source-primary mirror state. After finalization,
returning to the former backend is a new migration with a newly provisioned and
fully reconciled destination. It is not a rollback.

Source retirement is optional and is never performed by the application. It is
eligible only after the rollback window, acceptance, recovery proof, final
reconciliation, and explicit operator sign-off.

### Recovery

Every recovery point coordinates PostgreSQL with the backends required by live
blob service identities and the active migration phase:

- Disk-only: database plus Disk recovery reference.
- S3-only: database plus S3 recovery reference.
- Mirror or rollback window: database plus both storage references.

The existing 35-daily and 12-monthly retention objectives continue to apply.
Every accepted recovery point must pass an isolated restore, integrity check,
authorized retrieval, and cross-household denial check.

## Rejected alternatives

Making object storage mandatory would add infrastructure and credential
requirements for customers whose durable Disk topology is sufficient. Keeping
separate one-way migration implementations would duplicate verification,
rollback, and privacy behavior. Copying the Disk directory directly would copy
physical sharding rather than the Active Storage logical service contract.

## Consequences

Operators choose and provision a supported backend explicitly. Disk remains a
supported steady state. S3 enables storage-independent scheduling but adds
provider-owned durability, access-policy, lifecycle, and recovery duties.
Migration phases protect both backends and require maintenance gates. No
deployment may delete a source merely because application finalization passed.

## Related documents

- `docs/operations/upload-storage-backup-and-restore.md`
- `docs/operations/home-ops-portable-storage-handoff.md`
- `config/storage.yml`
- `config/environments/production.rb`
- GitHub issues #1551 and #1774
