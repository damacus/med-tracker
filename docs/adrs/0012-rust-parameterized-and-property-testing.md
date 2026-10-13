# ADR 0012: Rust parameterized and property-based testing

- Status
  Accepted
- Date
  2026-10-13

## Context

The Loco test-hygiene audit (#2575) noted the Rust suite had no parameterized or
property-based testing crates in dev-dependencies. Three candidates were
evaluated against how the suite is actually written:

- `proptest` — pure functions such as `nhs_dmd::extract::wildcard_match` and
  `unsafe_name` take arbitrary strings and have clear invariants (never panics,
  literal patterns match iff equal, `*` covers any single segment, `..`/absolute
  paths are always unsafe). These are classic property targets the existing
  hand-picked cases can miss.
- `rstest` — the `#[rstest]` case-table style targets the repeated
  schema-rejection and payload tables in `tests/care_api/`. Those tables are
  already expressed as `for` loops over `json!` case arrays, and each loop body
  shares one `Application`/`Fixture` — a per-database-clone + per-server-boot
  setup that is deliberately expensive. Splitting a loop into rstest cases
  re-runs that fixture per case; the gain is only per-case naming in output.
- `mockall` — the suite deliberately exercises real PostgreSQL for tenant
  isolation, RLS, and queue semantics. Mocks would weaken that coverage, and no
  mock-shaped seams exist in the code to hang them on.

## Decision

1. Adopt `proptest` for pure parser/matcher/codec functions. New property tests
   should state invariants (never-panic, round-trip, boundary) rather than
   re-enumerating examples. `nhs_dmd::extract` carries the first set.
2. Do not adopt `rstest`; keep writing case tables as `for` loops so each
   parameterized case shares its fixture setup.
3. Do not adopt `mockall`; tests exercise real PostgreSQL by design.
4. Audit items accepted as-is, no action: `#[should_panic]` assertions in the
   `contract.rs` schema helper (panicking *is* the helper's contract), the
   `#[ignore]`d `prepare_owned_bucket` provisioning hook, and `.unwrap()`
   density in tests (add `expect()` only where a failure message would be
   cryptic).

## Consequences

- `proptest` joins dev-dependencies pinned like the rest of the suite.
- Pure functions added to parsers, import normalization, and token/digest
  utilities should grow property tests rather than long example tables.
- Integration tests keep loop-based case tables; reviewers should not ask for
  rstest migrations.
