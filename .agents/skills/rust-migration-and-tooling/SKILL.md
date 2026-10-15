---
name: rust-migration-and-tooling
description: >-
  Guides fresh Rust/Loco workflows, Rust-native versioned portability packages, and strict Rust verification gates. Use when implementing features, working on Rust-native import/export, or running test suites.
---

# rust-migration-and-tooling

Read root AGENTS.md and ADRs 0013–0016 before applying this skill.
These are accepted direction, not evidence that pending application work passes.

- Preserve clinical records, medication history, care workflows, RLS/access,
  replay idempotency, atomic mail outbox/stock deduction, API v1/MCP contracts
  except explicit owner-approved changes. Do not preserve Rails schemas,
  Rodauth internals, sessions, account-person-user chains or internal JSON.
- Rust-native versioned portability pipeline: decrypt -> validate -> authorise
  -> preflight -> apply; FHIR is an external interoperability adapter; MCP is a
  tool trigger, never the migration format. No legacy import implementation.
- Historical restore never deducts current stock, enqueues live reminders/mail,
  or replaces clinical provenance with restore time.
- Bound spawn_blocking crypto with app-owned Semaphore; avoid needless clones.
- Use Task wrappers for all-target/all-feature Clippy -D warnings and machete;
  tests use nextest --no-tests=fail; cargo test only for doctests. No filtered
  false greens, zero tests must fail.
- proptest: replay no double stock deduction; restore no operational side
  effects; revoked membership/disclosure authority fails at boundary (the old
  intra-household person grants are replaced by whole-household member access);
  schedules across DST/timezone boundaries.

Use Fish/RTK and Task entry points. Apply TDD to executable changes.
No broad lint suppressions, including tests or generated entities.
Keep Rails read-only; no legacy data/credential import, parity or reverse migration.
Never reset or silently adopt a populated database. Preserve API/MCP contracts
except explicit access/location changes documented in OpenAPI, clients and ADRs.
