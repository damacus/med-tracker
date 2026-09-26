# Required dosage fields and household tokens: independent review

Reviewed by the orchestrator, independently of the implementation writer,
26 September 2026.

## Requirements

Pass. The migration and schema require amount, unit, frequency, maximum daily
doses, minimum hours between doses and dose cycle. Rust uses non-optional
types for the same six fields. Request validation still rejects absent create
values and explicit null values with 422. Optional inventory fields remain
optional. No values are fabricated and no special legacy-record path remains.

Household app tokens retain account, user, lockout, membership, permission
version, expiry and household checks. Their audit identity is distinct.
SMART integration credentials do not gain ordinary household API authority.

## Code quality and safety

Pass. PostgreSQL validates the NOT NULL constraints; Rails transactional DDL
rolls back earlier column changes when a later constraint fails. No data
backfill or deletion occurs. The migration failure names the affected column.
The migration specs cover each field, rollback and valid-value preservation.

Rust create-time required-value assertions follow successful required-field
validation. Updates preserve merged-record validation. Dose matching, stock
availability and stock removal use the newly required unit without a fallback.
The app-token change preserves existing API-session and mobile-OAuth branches.

No actionable correctness or security findings remain in the reviewed diff.
The separate non-operational issuing-household app-token HTTP case remains
explicitly unverified; passing tests do not imply full API completion.

## Evidence and rollout

The writer's `dosage-schema-report.md` records final commands and results.
The post-migration dosage HTTP run passed 13/13, including PostgreSQL 23502
rejection for all six fields and unchanged subsequent API reads. Focused Rails
migration tests passed 2/2; Rust check, clippy and 15 unit tests passed.

Run the schema migration successfully before deploying Rust code that requires
these fields. Production data has not been inspected or migrated. Full Rails
and lint results are publication gates, not inferred from the focused tests.
