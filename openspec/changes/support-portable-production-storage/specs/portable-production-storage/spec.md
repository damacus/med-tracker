## Purpose

Defines portable production blob storage so operators can run MedTracker with durable Disk or private S3-compatible storage and migrate safely between them without changing medication or attachment behavior.

## ADDED Requirements

### Requirement: Production storage backend selection is explicit and portable
The system SHALL support durable Disk and private S3-compatible storage as first-class production backends. The selected backend MUST be explicit, MUST validate its own required configuration, and MUST NOT require configuration for an unselected backend or silently fall back to another backend.

#### Scenario: Customer runs production without S3
- **GIVEN** production explicitly selects durable Disk storage and provides a valid writable persistent root
- **WHEN** the application and its storage-dependent jobs boot without S3 endpoint or credential settings
- **THEN** they use Disk storage successfully without contacting or requiring an S3-compatible service

#### Scenario: Customer runs production without a mounted blob volume
- **GIVEN** production explicitly selects private S3-compatible storage and provides valid endpoint, bucket, region, and credentials
- **WHEN** the application and its storage-dependent jobs boot without a persistent blob mount
- **THEN** they use the selected S3-compatible service successfully

#### Scenario: Selected backend configuration is invalid
- **GIVEN** the selected production backend is missing a required setting or cannot access its declared durable storage
- **WHEN** production storage configuration is validated
- **THEN** startup fails closed without exposing credentials or selecting an undeclared fallback

### Requirement: Supported backends preserve application and privacy behavior
Medication, attachment, export, and import behavior SHALL be independent of the selected production storage backend. Storage access MUST preserve household authorization and MUST expose private blobs only through authorized application behavior or short-lived private service access. Configuration, migration output, logs, traces, and metrics MUST NOT disclose credentials, object contents, original filenames, medication data, or person data.

#### Scenario: Authorized attachment access is backend-independent
- **GIVEN** an authenticated account is authorized to access a household attachment stored on either supported backend
- **WHEN** the account requests the attachment
- **THEN** the system returns the same authorized content without exposing the underlying storage location publicly

#### Scenario: Cross-household access is denied
- **GIVEN** an account is not authorized for the household that owns an attachment
- **WHEN** the account attempts to retrieve that attachment from either supported backend
- **THEN** access is denied without revealing whether the underlying blob exists

#### Scenario: Storage failure is reported safely
- **GIVEN** the selected backend cannot complete an upload, download, verification, or deletion
- **WHEN** the operation fails
- **THEN** diagnostics identify the operation and backend mode without disclosing protected content, filenames, credentials, checksums, household identifiers, or person identifiers

### Requirement: Queued storage inputs use durable service references
Every production job input that must survive beyond its originating request SHALL be persisted through the selected durable storage service before enqueue and referenced by opaque service identity and logical key rather than an application-local pathname. Storage-dependent jobs MUST work when every participating process can access the selected backend.

#### Scenario: NHS dm+d import crosses the request-job boundary
- **GIVEN** an authorized administrator submits a valid NHS dm+d release archive
- **WHEN** the import job is enqueued
- **THEN** the verified archive is represented by an opaque durable service reference rather than a process-local pathname

#### Scenario: Disk-backed worker accesses the declared durable root
- **GIVEN** production selects Disk and every process that performs storage work has access to the same declared durable root
- **WHEN** a storage-dependent job runs
- **THEN** it resolves the durable reference and completes without requiring S3

#### Scenario: S3-backed worker has no blob mount
- **GIVEN** production selects S3-compatible storage and a worker has no persistent blob mount
- **WHEN** it analyzes or purges an attachment, expires an export, or processes an NHS dm+d archive
- **THEN** it resolves the durable reference through the selected S3-compatible service

### Requirement: Migration is bidirectional, resumable, and idempotent
The system SHALL support verified Disk-to-S3 and S3-to-Disk migration using the same migration contract. A migration SHALL mirror new writes from the readable source to the destination, backfill every live source blob under its logical key, verify destination content before cutover, and allow interrupted phases to resume safely without creating duplicate logical blobs.

#### Scenario: Customer starts either migration direction
- **GIVEN** production is stable on Disk or S3-compatible storage and the other supported backend has been provisioned as a destination
- **WHEN** an operator starts a migration
- **THEN** the current backend remains the readable primary and new writes are mirrored to the destination

#### Scenario: Existing blob is backfilled
- **GIVEN** a live blob exists on the source and is absent from the destination
- **WHEN** the migration processes that blob
- **THEN** it writes the same logical key to the destination and verifies the destination content against the recorded blob checksum

#### Scenario: Migration resumes after interruption
- **GIVEN** a previous migration processed only part of the live blob set
- **WHEN** the operator resumes the same source-to-destination migration
- **THEN** already verified destination blobs are accepted without duplication and remaining blobs continue processing

#### Scenario: Concurrent mutation reaches a fixed point
- **GIVEN** uploads or deletions may occur during online backfill
- **WHEN** final reconciliation begins
- **THEN** storage mutations are quiesced, pending mirror work is drained, and reconciliation establishes a stable live set with no missing or corrupt destination blobs

### Requirement: Cutover, rollback, and future reverse migration preserve data
The system SHALL treat each blob's selected service as migration state, SHALL update the verified cutover set atomically, and SHALL preserve writes to both backends for a defined rollback window. Both migration directions MUST support in-window rollback. Completing one migration MUST NOT prevent an operator from provisioning the other backend later and performing a new reverse migration.

#### Scenario: Verification blocks cutover in either direction
- **GIVEN** one or more live blobs are absent from the destination or fail integrity verification
- **WHEN** an operator requests cutover
- **THEN** no blob service identities change and the source remains the readable primary

#### Scenario: Verified cutover enters rollback mode
- **GIVEN** every live blob is verified on the destination and pending mirror work is drained
- **WHEN** the cutover transaction completes
- **THEN** the destination becomes the readable primary and new writes continue to mirror to the source for the rollback window

#### Scenario: Cutover transaction rolls back
- **GIVEN** the transaction that changes blob service identities fails
- **WHEN** the database transaction rolls back
- **THEN** every blob retains its previous readable service and the cutover can be retried safely

#### Scenario: Operator rolls back during the window
- **GIVEN** the destination is primary and every post-cutover write has also been mirrored to the source
- **WHEN** an operator invokes rollback before the window closes
- **THEN** the source becomes the readable primary again without losing post-cutover blobs

#### Scenario: Customer later migrates from S3 back to Disk
- **GIVEN** production previously finalized on S3-compatible storage and no longer retains its former Disk source
- **WHEN** an operator provisions a new durable Disk destination and starts an S3-to-Disk migration
- **THEN** the system performs the same mirror, backfill, verification, cutover, and rollback contract used for Disk-to-S3 migration

### Requirement: Source retirement is explicit and optional
The system SHALL NOT require an operator to retire a healthy source backend after migration. Retirement SHALL require completed acceptance evidence, an expired rollback window, a final reconciliation, and explicit operator sign-off. Remaining on Disk indefinitely MUST remain a supported steady state.

#### Scenario: Customer remains on Disk
- **GIVEN** production uses a valid durable Disk backend and no migration is active
- **WHEN** the deployment is operated or upgraded
- **THEN** the application remains supported without an S3-compatible service

#### Scenario: Premature source retirement is rejected
- **GIVEN** a rollback window is open, final reconciliation has not passed, or a live workflow still depends on the source
- **WHEN** source retirement eligibility is evaluated
- **THEN** the source remains required and the unmet gate is reported safely

#### Scenario: Operator retains both backends after migration
- **GIVEN** migration and acceptance have succeeded
- **WHEN** the operator chooses not to retire the source
- **THEN** the application does not delete the source or its recovery history automatically

### Requirement: Backup and restore follow the selected storage state
The production recovery contract SHALL coordinate PostgreSQL records with every storage service required by live blob service identities. Disk-only recovery SHALL NOT require an object-storage backup, S3-only recovery SHALL NOT require a blob-volume snapshot, and an active migration or rollback window SHALL protect both participating backends. A recovery point MUST pass isolated verification before it is accepted as recoverable.

#### Scenario: Disk-only recovery set is recorded
- **GIVEN** production is stable on Disk with no migration active
- **WHEN** a recovery point is recorded
- **THEN** it identifies the database and durable Disk recovery points without requiring an S3 recovery reference

#### Scenario: S3-only recovery set is recorded
- **GIVEN** production is stable on S3-compatible storage with no migration active
- **WHEN** a recovery point is recorded
- **THEN** it identifies the database and private bucket recovery points without requiring a blob-volume snapshot

#### Scenario: Migration recovery set protects both services
- **GIVEN** migration or its rollback window is active
- **WHEN** a recovery point is recorded
- **THEN** it protects the database plus both participating storage services and records the active migration phase

#### Scenario: Restored attachment is verified
- **GIVEN** a recorded recovery set is restored into an isolated environment
- **WHEN** restore verification resolves a representative attachment through its recorded service
- **THEN** it proves integrity, authorized retrieval, and cross-household denial or rejects the recovery point

### Requirement: Deployment topology matches the selected backend
The deployment SHALL make the selected backend available to every process that performs storage work. Disk mode MUST NOT claim filesystem-independent scheduling unless those processes share the same durable filesystem. S3-compatible mode MUST support web and worker processes that are scheduled independently and have no shared blob mount.

#### Scenario: Disk topology is declared honestly
- **GIVEN** production selects Disk
- **WHEN** deployment compatibility is evaluated
- **THEN** each storage-dependent process is colocated with or mounted to the same durable root and no independent-scheduling claim is made without demonstrated shared access

#### Scenario: S3 topology supports independent processes
- **GIVEN** production selects S3-compatible storage
- **WHEN** web and worker processes are scheduled independently
- **THEN** both access the same private bucket without sharing a node-mounted blob filesystem

### Requirement: Development and test remain isolated
Development and test SHALL continue to use environment-local Disk services and MUST NOT require production S3 credentials, production mounted storage, or access to hosted buckets.

#### Scenario: Test execution is isolated
- **GIVEN** the automated test environment has no production storage credentials or mount
- **WHEN** tests create or destroy attachments
- **THEN** all blob operations remain inside the test-specific temporary Disk root

#### Scenario: Development remains self-contained
- **GIVEN** a developer has not explicitly configured an external storage integration
- **WHEN** the development application starts
- **THEN** it uses its local development Disk service without contacting a production backend
