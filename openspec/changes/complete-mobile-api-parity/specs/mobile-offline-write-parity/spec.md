## Purpose

Let native clients replay supported care and inventory writes safely while keeping identity and access changes online.

## ADDED Requirements

### Requirement: Advertise an explicit operation matrix
The API SHALL advertise and document the supported offline operations listed in design.md, preserving existing operations and rejecting identity, profile, avatar and access changes as online-only.

#### Scenario: Discover replay support
- **GIVEN** a client reads capabilities
- **WHEN** it prepares an offline queue
- **THEN** it can identify each supported action/resource pair

#### Scenario: Reject online-only actions
- **GIVEN** a queued invitation, profile or membership/access action
- **WHEN** the batch is replayed
- **THEN** a stable unsupported-operation error is returned without mutation

### Requirement: Replay through current domain rules
Supported offline writes SHALL enforce current authorization, shared domain validation, idempotency, ETags where mutable, deterministic results, and all-or-nothing batch rollback including audit and sync effects.

#### Scenario: Replay after revocation
- **GIVEN** an operation was queued while access existed and access is now revoked
- **WHEN** the client submits the queue
- **THEN** the operation is denied without mutation or disclosure

#### Scenario: Roll back a failed batch
- **GIVEN** an earlier operation is valid but a later one conflicts
- **WHEN** the batch is submitted
- **THEN** none of its writes or audit/sync changes persist

#### Scenario: Replay a committed batch
- **GIVEN** the server committed but the client lost its response
- **WHEN** the same idempotent batch is retried
- **THEN** the prior result is returned and stock/outcomes are not repeated

#### Scenario: Reject stale mutable state
- **GIVEN** an offline update contains a superseded ETag
- **WHEN** it is replayed
- **THEN** a conflict preserves server changes
