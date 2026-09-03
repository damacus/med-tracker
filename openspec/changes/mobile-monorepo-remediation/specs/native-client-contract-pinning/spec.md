## Purpose

Allows native releases to consume a reproducible server contract while retaining the repository OpenAPI document as the sole authoritative Rails API description.

## ADDED Requirements

### Requirement: Each native client pins its API contract
Each native client SHALL keep a copied OpenAPI document, source revision, source checksum, and checked-in generated client artifacts derived from `docs/api/openapi.v1.yaml`.

#### Scenario: Contract pin update
- **WHEN** a developer runs the client's root contract-update task
- **THEN** the authoritative document is copied, provenance and checksum are updated, client artifacts are regenerated, and drift validation runs

#### Scenario: Independent client release
- **WHEN** the authoritative Rails contract changes without updating a client's pin
- **THEN** the existing client remains reproducible from its pinned contract and no generated client files change implicitly

### Requirement: Contract drift fails native CI
Each native client SHALL provide a non-mutating contract check that fails when its checked-in generated artifacts do not match its pinned OpenAPI document.

#### Scenario: Generated source differs
- **WHEN** checked-in generated source differs from fresh generation using the pinned contract
- **THEN** the native CI task fails and identifies contract drift
