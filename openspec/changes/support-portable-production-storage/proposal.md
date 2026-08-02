## Why

Production storage is currently coupled to a Longhorn `ReadWriteOnce` volume, while the proposed S3 migration would make object storage a permanent requirement. GitHub issue [#1774](https://github.com/damacus/med-tracker/issues/1774) needs a portable storage boundary instead: operators must be able to run MedTracker with durable Disk storage or S3-compatible storage and migrate safely in either direction without changing application behavior.

## What Changes

- Keep durable Disk storage as a supported production configuration that does not require S3 credentials or an object-storage service.
- Add an optional private S3-compatible production configuration suitable for independently scheduled web and background processes.
- Introduce an idempotent, resumable migration boundary that can move live blobs from Disk to S3-compatible storage or from S3-compatible storage to Disk, verifies the destination before cutover, and retains the source through an explicit rollback window.
- Make queued storage inputs, including NHS dm+d release archives, use service-agnostic durable blob references instead of application-local pathnames.
- Define backup, isolated restore, privacy-safe diagnostics, and deployed acceptance contracts for either selected production backend.
- Preserve isolated Disk services for development and test.
- Amend ADR 0008 to describe Disk as one supported production topology and document the operational capabilities and constraints of each backend.

This change does not require S3 for self-hosted or customer installations, introduce NFS or RWX storage, replace Solid Queue, or require web and background processes to be deployed separately. Independent process scheduling remains available only when the selected durable backend is reachable by both processes.

## Capabilities

### New Capabilities

- `portable-production-storage`: Defines explicit Disk and S3-compatible production modes, bidirectional verified migration, service-agnostic queued inputs, backend-aware recovery, and safe source retirement.

### Modified Capabilities

None. The repository has no existing main OpenSpec capability for production blob storage.

## Impact

- Rails Active Storage configuration and production storage validation.
- NHS dm+d import persistence and job execution.
- Bidirectional migration, reconciliation, cutover, rollback, backup, restore, and production acceptance tooling.
- ADR 0008 and self-hosted deployment documentation.
- The `home-ops` deployment may provision private S3-compatible buckets and identities when that mode is selected, but S3 infrastructure and credentials remain optional and infrastructure edits remain outside this repo-local change.
