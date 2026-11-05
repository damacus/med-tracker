## Purpose

Extends DM+D import reliability so every failed import records a typed failure
category, a safe stored summary and a trace reference the administrator can
use to find the corresponding logs and traces.

## ADDED Requirements

### Requirement: Failed imports record a typed failure category and trace reference

Each import run SHALL execute inside a trace and SHALL store its trace
identifier (32 lowercase hex) on the import record at run start. A failed
import SHALL record a typed failure category (`archive_invalid`,
`xml_invalid`, `storage`, `database`, `timeout`, `cancelled`, `panicked`,
`stale`) and a fixed safe summary as its stored message; raw error text SHALL
NOT be stored or displayed. The failed-import page SHALL display the trace
reference to the administrator.

#### Scenario: Import fails at a stage

- **GIVEN** an import run fails during a stage
- **WHEN** the failure is persisted
- **THEN** the record contains the stage's typed category and a fixed safe
  summary
- **AND** raw error text is absent from the stored message, the page, the
  logs and the exported span

#### Scenario: Administrator views a failed import

- **GIVEN** a failed import record
- **WHEN** the administrator opens the failed-import page
- **THEN** the stored trace reference is displayed so the operator can find
  the corresponding logs and traces

#### Scenario: Import is reconciled as stale

- **GIVEN** an active import passes the staleness boundary
- **WHEN** reconciliation marks it failed
- **THEN** the stored failure category is `stale` and the trace reference
  remains available
