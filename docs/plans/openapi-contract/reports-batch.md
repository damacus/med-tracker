# Report contract tests

## Scope

Cover the four fixed OpenAPI operations: health-history JSON/PDF and medication-review JSON/PDF. Reuse the existing report fixtures and test helpers; do not change production or runner files.

## Existing coverage

`rust/contract-tests/tests/reports.rs` exercises report contents and filters, PDFs, no-store headers, authorization scopes, invalid parameters, and success-only download audits.

## Remaining checks

- Validate the closed JSON report envelopes and nested report fields against the OpenAPI schemas.
- Prove unauthenticated access is rejected on each JSON/PDF route.
- Exercise invalid filters on both PDF routes.
- Verify rendered clinical content through extracted PDF text.
- Exercise the documented renderer-failure 503 through Sol's isolated missing-font service.

## Out of scope

Do not duplicate existing chronology, status filtering, PDF rendering, or household/record visibility cases. No production or shared runner changes are part of this test batch.
