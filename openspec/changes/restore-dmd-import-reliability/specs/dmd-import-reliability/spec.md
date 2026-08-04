## Purpose

Ensures DM+D releases report trustworthy live progress, execute within bounded isolated resources, recover from interruption, and pass a canary gate before production promotion.

## ADDED Requirements

### Requirement: Active imports update without manual reload
The system SHALL update an active DM+D import page when committed progress or terminal state changes occur, without requiring a manual page reload or inline executable script.

#### Scenario: Import progress is committed
- **GIVEN** an administrator is viewing the active DM+D import
- **WHEN** the import commits updated progress
- **THEN** the displayed progress refreshes without manual navigation
- **AND** the page remains compatible with the production content security policy

#### Scenario: Import reaches a terminal state
- **GIVEN** an administrator is viewing an active DM+D import
- **WHEN** the import commits a completed or failed state
- **THEN** the page refreshes to show the terminal result and final log

### Requirement: Only one import may be active
The system SHALL permit at most one DM+D import in a queued, extracting, counting, or importing state and SHALL preserve that invariant under concurrent requests.

#### Scenario: No import is active
- **GIVEN** no DM+D import is active
- **WHEN** an administrator submits a valid release archive
- **THEN** one import is accepted for processing

#### Scenario: Another import is active
- **GIVEN** a DM+D import is already active
- **WHEN** another administrator submits a release archive
- **THEN** the second import is not activated
- **AND** the administrator receives a clear notice identifying that another import is in progress

### Requirement: Interrupted imports are reconciled
The system SHALL mark an active DM+D import as failed when it has not been updated for 30 minutes and SHALL perform that reconciliation at least every five minutes.

#### Scenario: Active import is stale
- **GIVEN** an active DM+D import has not been updated for at least 30 minutes
- **WHEN** reconciliation runs
- **THEN** the import is marked failed
- **AND** its log records that the import was interrupted

#### Scenario: Active import is recent
- **GIVEN** an active DM+D import was updated less than 30 minutes ago
- **WHEN** reconciliation runs
- **THEN** its state and log remain unchanged

### Requirement: Import execution is isolated from the web process
Production and canary deployments SHALL execute DM+D imports in a dedicated job worker that uses the same application release, configuration, secrets, and durable archive storage as the web process while retaining an independent memory limit.

#### Scenario: Import worker exceeds its memory limit
- **GIVEN** a DM+D import is executing in the dedicated worker
- **WHEN** that worker is terminated or restarted because it exceeds its memory limit
- **THEN** the web process remains available and does not restart because of the worker failure
- **AND** stale-import reconciliation can move the interrupted import to a failed state

#### Scenario: Worker processes an uploaded archive
- **GIVEN** the web process has accepted and durably stored a DM+D release archive
- **WHEN** the dedicated worker executes the queued import
- **THEN** it uses the same application version and can access the stored archive

### Requirement: Import memory remains bounded by the worker budget
Long-running DM+D imports SHALL avoid retaining per-lookup database cache entries so a full public release can complete within the dedicated worker's one GiB memory limit.

#### Scenario: Full archive performs unique barcode lookups
- **GIVEN** a full DM+D release contains hundreds of thousands of distinct barcode lookups
- **WHEN** the worker imports the archive
- **THEN** database lookup results are not accumulated in a request-style query cache
- **AND** the worker remains within its configured memory limit or fails independently from the web process

### Requirement: Canary gates production promotion
The deployment SHALL NOT promote the worker topology to production until the same application image and worker contract complete a full public DM+D release in canary.

#### Scenario: Canary import succeeds
- **GIVEN** canary is running the candidate application image and dedicated worker topology
- **WHEN** a full public DM+D release is imported
- **THEN** progress changes without manual reload
- **AND** the worker remains within its memory limit
- **AND** the web process remains healthy without restarts or failed probes
- **AND** the import reaches completed with final counts and log matching persisted data

#### Scenario: Canary acceptance fails
- **GIVEN** any canary import acceptance invariant fails
- **WHEN** production promotion is considered
- **THEN** production retains its existing deployment state
- **AND** the failure is corrected in the repository and pull-request boundary that owns it

### Requirement: Canary preserves independent storage and evidence boundaries
The canary acceptance environment SHALL preserve durable application archive behavior while remaining free of production-derived database recovery and database backup/archive state. The acceptance record SHALL distinguish application blob storage from database backup storage and SHALL retain PHI-safe evidence for DM+D execution and the unresolved medication-administration persistence observation.

#### Scenario: Canary storage contract is verified
- **GIVEN** canary is rebuilt from its synthetic baseline
- **WHEN** its storage and reset contracts are validated
- **THEN** application release archives remain durably accessible to the worker
- **AND** removal of database backup or archive state is not treated as removal of application blob storage
- **AND** the acceptance record identifies the selected application storage mode without exposing credentials or household data

#### Scenario: Reminder persistence evidence is collected
- **GIVEN** a synthetic canary user records a medication administration and exercises a reminder scenario
- **WHEN** operational acceptance captures evidence
- **THEN** the record distinguishes persisted administration data from notification-event and delivery evidence
- **AND** failures retain PHI-safe logs and traces for a separately owned follow-up rather than being silently attributed to DM+D import behavior

### Requirement: Delivery is agent-orchestrated and independently reviewed
The MedTracker-owned change SHALL be delivered through a coordinating parent agent that retains architecture, cross-repository scope, repository ownership, stacked-branch lineage, integration, and final verification authority. The parent SHALL implement the production capability across both the MedTracker application repository and its deployment repository while preserving each repository's local instructions, code ownership, commits, and pull-request lineage. Each bounded implementation task SHALL be assigned to a fresh subagent with explicit acceptance criteria and SHALL receive an independent task review before the next task begins.

#### Scenario: Production delivery crosses the repository boundary
- **GIVEN** the MedTracker capability requires both application behavior and deployment topology to ship
- **WHEN** the coordinating agent applies this change
- **THEN** this MedTracker OpenSpec change remains the single planning authority and progress ledger
- **AND** application work is committed in the MedTracker repository
- **AND** deployment work is committed in the deployment repository under that repository's own instructions
- **AND** MedTracker artifacts do not encode deployment-repository filesystem or manifest paths

#### Scenario: Bounded task is dispatched
- **GIVEN** a task has explicit scope, owned files, constraints, acceptance evidence, and escalation conditions
- **WHEN** the coordinating agent dispatches implementation
- **THEN** the selected subagent receives a task-specific brief and writes a durable report with changed files, verification, and concerns
- **AND** the coordinating agent records reviewed completion in the progress ledger

#### Scenario: Task introduces cross-cutting risk
- **GIVEN** a task exposes concurrency, deployment-control, destructive-state, public-contract, or stack-lineage ambiguity
- **WHEN** the implementing or reviewing subagent reports that ambiguity
- **THEN** the coordinating agent retains or resumes judgment using the highest necessary reasoning tier
- **AND** no parallel implementation task proceeds until the decision and affected task briefs are updated

#### Scenario: Task implementation is complete
- **GIVEN** an implementing subagent reports a bounded task complete
- **WHEN** the task diff and verification evidence are available
- **THEN** an independent reviewer evaluates specification compliance and task quality against the task brief
- **AND** critical or important findings are corrected and re-reviewed before another implementation task begins
