# ADR 0008: Production Upload Storage

- Status: Accepted
- Date: 2026-07-10

## Context

Active Storage previously selected the development-oriented `local` service in
production. Docker Compose happened to mount a named volume at `/app/storage`,
while the hosted Kubernetes deployment used a 5 Gi Longhorn `ReadWriteOnce`
claim. Rails did not distinguish that durable mount from the writable directory
inside an application image, so a missing mount could silently accept uploads
that disappeared with the pod.

The hosted deployment already has one web replica, uses the `Recreate` strategy,
and cannot mount its `ReadWriteOnce` claim from multiple nodes. That topology is
deliberately single-node.

## Decision

Production uses the `persistent` Active Storage disk service rooted at
`ACTIVE_STORAGE_ROOT`, which defaults to `/app/storage`.
`ACTIVE_STORAGE_SERVICE` may select only `persistent`. At runtime the application
verifies that the root is absolute, exists, is writable, and is listed as a mount
point by the Linux kernel. Production boot fails when any check fails. Asset
compilation with `SECRET_KEY_BASE_DUMMY=1` skips only the runtime mount check.

The deployment contract is:

- one web replica;
- a `ReadWriteOnce` persistent volume;
- `Recreate` deployment strategy;
- coordinated backups of PostgreSQL Active Storage records and `/app/storage`;
- encrypted off-cluster retention and regular isolated restore tests.

Horizontal web scaling is not supported by this storage contract. Scaling web
replicas requires either an RWX storage service with demonstrated cross-node
semantics or a migration to object storage before replicas are increased.

## Rejected Alternative: Object Storage

Object storage is a strong future choice for portable, horizontally scaled
deployments, but adopting it now would add an S3 provider dependency, secrets,
bucket lifecycle policy, and a live blob migration while the deployed topology
remains single-replica. That operational expansion is not needed to make the
current Longhorn-backed service durable and fail closed.

Object storage remains the preferred migration path when horizontal scaling or
cross-cluster portability becomes a requirement. Rails mirror services can be
used during that migration, followed by a verified copy and service cutover.

## Consequences

Uploads cannot be accepted when the production volume is absent or mounted at
the wrong path. Compose, Kamal, and hosted Kubernetes must all mount the same
root. Operators must back up the database and blob volume as one recovery set.
Deployments remain single-replica until the storage decision changes.

## Related Documents

- `docs/operations/upload-storage-backup-and-restore.md`
- `config/storage.yml`
- `config/environments/production.rb`
- GitHub issue #1551
