# ADR 0016: Fresh Rust without legacy compatibility

- Status
  Accepted (architecture; implementation acceptance pending)
- Date
  2026-10-15

## Context

Earlier migration plans aimed at Rails parity, database adoption and rollback.
The owner now requires fresh Rust. Those assumptions are superseded for this
delivery; historical records describe prior work, not current acceptance rules.

## Decision

Use Loco 1.2, Tera/DaisyUI and PostgreSQL 18. Rails is read-only reference.
There is no legacy data import, credentials migration, database parity, reverse
migration or production deployment. Fresh-schema commands must never reset or
silently adopt populated databases.

Use typed request DTO → validated command → operation → typed result → response
DTO boundaries throughout root `src/`. Use distinct entity ID newtypes, enum dose
sources, typed conflict codes, parsed timezone-aware timestamps and exact Decimal
math. JSON is valid for transport, configuration, templates and document storage;
internal struct → Value → struct command passing is prohibited.

Rust-native versioned portability remains in scope: decrypt, validate, authorise,
preflight, apply. Historical restore must not deduct stock, enqueue live reminders
or mail, or replace clinical provenance with restore time. FHIR is an external
adapter; MCP triggers tools and is not the portability format. Property tests must
cover replay without double deduction, side-effect-free restore, revoked authority
and DST/timezone schedules. Preserve existing API/MCP contracts except the explicit
access/location changes, documented together in OpenAPI, clients and ADRs.

Retain older `rust/` crates. Root Task retains `rust/Taskfile.yml` and
`rust/api/Taskfile.yml` for explicit reference work. The root CI workflow tests
the active Loco application and client tools; it does not run the superseded
dashboard or Rails compatibility suites. Older crates remain tooling/test
consumers and historical inputs; the Axum/Leptos application is superseded by
root Loco routing. Client tools form their own workspace and use their shared
API client, not these legacy application crates. Do not delete them as dead code
without rechecking consumers.

## Alternatives

Rails parity and legacy-schema import add work expressly removed by the owner.
Deleting all older Rust sources breaks retained tooling consumers. Neither is
part of this decision.

## Consequences

Tests and guidance must follow fresh Rust decisions rather than encode abandoned
parity. Run focused tooling checks and documentation build for guidance changes;
application delivery still needs its own TDD and full relevant gates. No merge,
deployment or live-data action follows from architecture acceptance.
