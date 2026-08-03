## Context

See `proposal.md` for motivation and `specs/portable-production-storage/spec.md` for the behavior contract. ADR 0008 currently permits only the `persistent` Active Storage Disk service at `/app/storage` and couples hosted production to one mounted `ReadWriteOnce` volume. `ProductionStorage` rejects every other service and verifies that the Disk root is a writable kernel mount.

Active Storage records a `service_name` on each blob. Changing only the environment default affects new blobs but neither moves existing bytes nor changes how existing records resolve their service. Rails 8.1 Mirror service writes to its primary synchronously and mirrors new uploads asynchronously; historical blobs still require an explicit backfill and integrity proof.

The NHS dm+d importer also persists a pathname below the application storage directory across its request-to-job boundary. That path is process-local unless all participating processes share the same filesystem. The replacement boundary must use the configured storage service without weakening the household-scoped Active Storage attachment model for this global administrative import.

The cluster provides RustFS, but that deployment detail must remain optional. The application owns backend selection, portable references, migration state, verification, and recovery contracts. Deployment repositories own mounts or buckets, identities, secret delivery, scheduling, and source retirement.

## Goals / Non-Goals

**Goals:**

- Treat durable Disk and private S3-compatible services as supported production steady states.
- Provide one symmetric migration mechanism for Disk-to-S3 and S3-to-Disk moves.
- Preserve reads and writes during bounded online backfill and quiesced cutover phases.
- Keep storage selection out of medication, attachment authorization, export, and import behavior.
- Make migration, rollback, recovery, and retirement operator-controlled and PHI-safe.
- Describe the deployment capability gained or constrained by each backend without imposing one topology on all operators.

**Non-Goals:**

- Requiring an S3-compatible service for production, development, or test.
- Adding GCS, Azure Blob, NFS, RWX, or another storage adapter in this change.
- Replacing Solid Queue or requiring separate web and worker deployments.
- Automatically deleting a source backend or its recovery history.
- Changing household export contents, NHS dm+d import semantics, or attachment authorization.

## Decisions

### 1. Keep Disk and S3 as explicit first-class steady modes

`ProductionStorage` will accept two steady modes:

- `persistent`, backed by the existing mounted durable root and retaining the current service name; and
- `s3`, backed by an explicitly configured private S3-compatible endpoint and bucket.

Disk validates its absolute, writable persistent mount and ignores absent S3 configuration. S3 validates its required non-secret inputs and credentials but does not require `/app/storage`. Neither mode detects or falls back to the other automatically. Development and test retain their existing local Disk services.

The hosted deployment may choose RustFS, while another customer may remain on a mounted Disk backend indefinitely or select another compatible S3 provider.

**Rejected alternatives:** making S3 the final mandatory mode would prevent simple single-process installations; silently falling back to Disk could accept uploads into an ephemeral image; automatically selecting a backend from available credentials would make boot behavior and recovery state ambiguous.

### 2. Treat scheduling independence as a backend capability

Disk remains valid when every process performing storage work accesses the same durable root. That can be one Puma process with embedded Solid Queue, multiple containers in one pod sharing the volume, or an operator-provided shared filesystem with separately proven semantics. The application will not claim that an RWO-backed Disk deployment supports independently scheduled pods.

S3 mode removes the shared mount requirement and therefore permits independently scheduled web and worker processes. Splitting those processes remains a separate deployment decision.

**Rejected alternative:** forcing a worker split as part of storage portability would increase baseline memory and make a deployment topology, rather than application behavior, mandatory.

### 3. Use a symmetric four-state migration machine

Define four production service states:

1. `persistent`: Disk is the sole service and existing production baseline.
2. `persistent_with_s3_mirror`: Disk is readable primary and S3 is mirror.
3. `s3_with_persistent_mirror`: S3 is readable primary and Disk is mirror.
4. `s3`: S3 is the sole service.

Disk-to-S3 follows states 1 → 2 → 3 → 4. S3-to-Disk follows 4 → 3 → 2 → 1. An operator can enter a new reverse migration after either source was previously retired by provisioning a fresh destination and starting from the corresponding steady state. Retaining `persistent` avoids rewriting existing blob service identities merely to keep using the already-supported Disk backend.

Each state is selected explicitly. Disk-inclusive states validate the mount; S3-inclusive states validate object-storage inputs. The two mirror states are normal migration states in either direction, not one-way special cases.

**Rejected alternative:** separate migration implementations for each direction would duplicate reconciliation, retry, transaction, and privacy behavior and make reverse migration more likely to drift.

### 4. Backfill and reconcile by logical service key

A direction-neutral migration run records its opaque run id, source service, destination service, phase, and aggregate progress. It iterates live blob records using the owner-capable operational database role in bounded batches. For each blob it checks the logical key on the destination, copies from the source when absent, and opens the destination with the recorded checksum. An existing valid destination object is accepted, which makes retries idempotent.

The mirror state covers new uploads during online backfill, but enqueue success is not replication proof. The cutover gate therefore:

1. stops attachment mutations and storage-dependent job dispatch;
2. drains pending mirror, analyze, and purge work;
3. reconciles a stable live blob set;
4. requires zero missing or corrupt destination blobs; and
5. atomically changes that stable set to the destination-primary mirror service.

If the database transaction rolls back, every blob retains its prior readable service. Destination copies remain safe and the migration can resume.

**Rejected alternatives:** changing the default service alone strands existing blobs; relying only on Mirror service omits historical backfill; copying the Disk directory directly preserves Disk sharding rather than service logical keys and does not update blob service identities.

### 5. Separate rollback from future migration

During a cutover rollback window, writes continue to both services and rollback atomically returns the verified set to the source-primary mirror state. Finalization changes the set to the destination steady service only after the window, acceptance, recovery proof, and final reconciliation.

After finalization, returning to the former backend is not described as rollback. It is a new migration with a newly validated destination, full backfill, cutover, rollback window, and recovery proof. This distinction prevents operators from assuming that retired source data is still current.

Source retirement is an explicit deployment action, never an application side effect. Keeping both storage systems after finalization remains valid, although the inactive copy is not assumed current unless a new migration reconciles it.

### 6. Persist queued inputs as service-neutral references

The NHS dm+d request persists an opaque service name, logical key, checksum, and byte size before enqueue. A small archive-store boundary resolves that reference through the selected Active Storage service API. It does not create a household-less attachment or weaken household RLS.

Existing queued path-backed imports must finish or be converted before a deployment removes their required filesystem. The same reference works with Disk or S3; only the service resolver changes.

### 7. Keep operator evidence backend-aware and PHI-safe

Provide bounded commands for starting or resuming migration, reconciliation, cutover eligibility, cutover, rollback, finalization, and source-retirement eligibility. Material mutations default to dry-run and require explicit source, destination, phase, and confirmation inputs. Output contains aggregate counts and opaque run ids, never filenames, contents, credentials, checksums, household ids, person ids, or medication data.

Acceptance must prove Disk-only operation without S3 settings, S3-only operation without a mounted blob volume, and a canary round trip from Disk to S3 and back to Disk. This is where bidirectionality becomes deployed evidence rather than an untested configuration promise.

### 8. Build recovery sets from recorded service state

Recovery tooling resolves the set of services required by live blob records and the active migration phase:

- Disk steady state coordinates PostgreSQL with the durable filesystem recovery point.
- S3 steady state coordinates PostgreSQL with the protected bucket recovery point.
- Mirror and rollback states protect both services until finalization.

The isolated restore verifier resolves blobs through their recorded service, verifies integrity, and proves authorized and cross-household behavior. Existing daily and monthly retention objectives remain unchanged. Off-cluster protection and provider-specific backup mechanics remain deployment responsibilities.

ADR 0008 will be amended rather than superseded: its fail-closed Disk rules remain the Disk-mode contract, while the follow-up records explicit backend selection, S3 mode, portability, and the limits of each deployment topology.

## Risks / Trade-offs

- **[A mirror job lags or fails before cutover]** → Drain storage queues and require stable full reconciliation rather than treating enqueue success as replication proof.
- **[Reverse migration is treated as an untested rollback path]** → Exercise a complete Disk-to-S3-to-Disk canary round trip and use the same state machine and commands in both directions.
- **[Blob service names continue selecting the old primary]** → Make service-name distribution an explicit invariant and change the verified set atomically.
- **[Concurrent deletion races with backfill]** → Permit online copying but establish the final live set only while mutations and storage jobs are quiesced.
- **[Disk mode is deployed with an inaccessible worker filesystem]** → Validate the durable root in each storage-dependent process and document that independent scheduling requires demonstrated shared access.
- **[S3-compatible behavior differs between providers]** → Verify upload, existence, download, integrity, delete, private access, and failure behavior against the operator's actual service before cutover.
- **[A source is assumed current after finalization]** → Distinguish in-window rollback from a future reverse migration and require a new full reconciliation.
- **[Migration reads health-related content]** → Restrict execution to the owner-capable operator boundary and emit only aggregate PHI-safe evidence.
- **[Recovery captures the database and wrong blob generation]** → Record backend-specific recovery references and reject the recovery point until isolated verification passes.

## Migration Plan

1. Add both steady modes, both mirror modes, service-neutral queued inputs, direction-neutral migration commands, and backend-aware recovery while production remains in its current Disk state.
2. Prove the unchanged Disk-only configuration in a final production image without S3 settings.
3. In canary, provision an isolated S3-compatible destination, select `persistent_with_s3_mirror`, backfill, reconcile, and cut over to `s3_with_persistent_mirror`.
4. Exercise in-window rollback to Disk, repeat cutover, prove S3-only behavior without `/app/storage`, and finalize to `s3` without automatically deleting the Disk source.
5. Provision a fresh durable Disk destination and run the full reverse path: `s3` → `s3_with_persistent_mirror` → `persistent_with_s3_mirror` → `persistent`, including a rollback exercise and isolated restore.
6. Record the round-trip evidence and update ADR 0008 and deployment documentation. Production remains on its selected backend until an operator separately approves a migration.
7. For any later customer migration, repeat the proven directional sequence, retain the source throughout its rollback window, and retire it only after explicit sign-off.

Before finalization, rollback selects the source-primary mirror state and atomically restores source-readable service identities. After finalization, moving back uses a new migration rather than an in-place rollback.
