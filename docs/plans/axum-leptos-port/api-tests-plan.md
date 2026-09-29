# API parity test execution plan

Spec: [plan.md](plan.md) and [api-tests-brief.md](api-tests-brief.md).

## Global constraints

- Write black-box tests from the current observable Rails behaviour and
  `docs/api/openapi.v1.yaml` before writing Rust product routes.
- API and browser tests are target-independent. Rails must pass as the
  baseline; Rust failures are recorded until implementation follows.
- Use `rust/` for the new test code and coverage matrix. Preserve the Rails
  root, database schema, native projects, and existing OpenAPI contract.
- Use isolated, disposable test data. Never point write tests at shared or
  production databases. Do not add production authentication bypasses.
- Keep one active writer. Every task gets independent requirements and quality
  review, fixes from the same writer, exact checks in its report, and a ledger
  entry before the next task.
- Follow Fish syntax and the repository `task` command boundary. Use `rtk`
  for shell commands. Do not add or remove source comments.
- Check author, committer, and signing identity as
  `Dan Webb <dan.webb@damacus.io>` before each commit. Do not change signing
  configuration globally. The controller owns branch push and handoff.

## Task 1: Inventory operations and test sources

Produce `rust/parity-matrix.md` with an entry for each of the 118
method/path operations currently in `docs/api/openapi.v1.yaml`. Include
operation ID, HTTP method/path, security requirement, source request spec,
planned black-box test file, and status. Group the non-v1 public HTTP
surfaces from `config/routes.rb` separately: OAuth, FHIR R4, MCP, web offline,
PWA, uploads, and platform administration. Mark any endpoint whose retention
or contract is unclear for a controller ruling; do not silently omit it.

Add a machine-readable inventory of every existing `spec/**/*_spec.rb` file,
with a provisional category: API, browser, Rust domain, operational, or
Rails-only. Give each Rails-only item a rationale and flag uncertain ones for
review. This inventory is a coverage map, not an assertion that a test has
already been ported. Check the operation and spec counts against the source
tree. Verify Markdown with `task docs:build` and whitespace with
`git diff --check`; report any environment blocker accurately.

## Task 2: Establish the black-box runner and first red cases

Create a test-only `rust/contract-tests/` crate and Taskfile commands for
running it against an isolated Rails URL and later a Rust URL. The command
must refuse a non-local or unapproved target by default. Provision disposable
fixtures without a product-only auth bypass. Implement an initial public
capabilities case, one authenticated read, and one denied cross-household
case from the Task 1 matrix. Run them against Rails and record the green
baseline. Run the same cases against the absent Rust target and record the
expected red result without misreporting it as a product regression.

## Task 3: Authentication and response envelopes

Port cases for capabilities, OAuth discovery and PKCE/token flows, sessions,
households, account state, access failure, validation errors, correlation
IDs, and rate limiting. Cover token expiry and revocation. Run all cases
against Rails, update the matrix, and review the exact HTTP outputs. Keep
web-only login journeys for the UI test phase.

## Task 4: Care and medication records

Port API cases for people, grants, locations, medications, dosage options,
health events, schedules, direct assignments, dose occurrences, takes,
stock, and reviews. Cover successful reads and writes, invalid data,
cross-household non-disclosure, person-scoped permissions, pagination,
ETags, decimal/timestamp precision, stock mutation, immutable history,
and audit effects. Run the cases against Rails and update the matrix.

## Task 5: Sync, offline, and replay

Port snapshot, change-feed, batch, mobile snapshot, offline dose replay,
idempotency, conflict, tombstone, and permission-recheck cases. Verify
whole-batch rollback, concurrent duplicate submissions, and revocation
between an original response and a cached replay. Run against Rails and
update the matrix.

## Task 6: Administration and remaining integrations

Port administration, invitation, exports, reports and PDFs, portable
import/export, avatars, attachments, notifications, device tokens, lookup,
FHIR R4, MCP, and other retained public routes. Cover security and failure
paths as well as success. For external services, use deterministic adapters
at the test environment boundary without exposing test controls publicly.
Run against Rails and update the matrix.

## Task 7: API corpus review and handoff

Independently review the full API matrix against every OpenAPI operation,
retained non-v1 route, and relevant existing request and domain specs. Resolve
missing or duplicate coverage. Verify the complete API suite against Rails
and report expected Rust red status separately. Produce the UI test phase
handoff with fixtures, commands, known gaps, and unresolved rulings.

For the retained web AI suggestion action, Rust parity requires two separate
server configurations: run `rtk task contract:web-json-actions-rust` against an
enabled target and `rtk task contract:web-json-actions-disabled-rust` against a
feature-disabled target. The generic Rust contract run ignores the disabled
case and cannot establish feature-gate parity by itself. Count this route as
ported only when both commands pass against their matching configurations.
