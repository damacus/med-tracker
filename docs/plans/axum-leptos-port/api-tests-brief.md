# API test writing brief

Owner: one implementation subagent per bounded task, with an independent
review after each task. The controller owns integration and acceptance.

## Result

Produce a target-independent HTTP test suite under `rust/contract-tests/` and
an operation-by-operation matrix under `rust/parity-matrix.md`. Translate
observable behaviour from `docs/api/openapi.v1.yaml`,
`spec/requests/api/v1/`, other relevant request specs, and public integration
routes. Run the suite against an isolated Rails test instance. Preserve
fixtures and failures as evidence; do not implement Rust server routes yet.

## Task order

1. Inventory each v1 method/path, response schema, auth requirement, source
   request specs, and currently missing behaviour case. Inventory retained
   FHIR, MCP, OAuth discovery/token, offline, PWA, and upload endpoints
   separately. Classify all RSpec files for later API/browser/domain review.
2. Establish disposable data and a task-wrapped black-box runner. Prove one
   public endpoint, one authenticated endpoint, and one denied case against
   Rails. Test setup must be idempotent and must never touch shared data.
3. Add auth, capabilities, session, household, error envelope, correlation,
   and rate-limit cases.
4. Add people, grants, locations, medications, dosage options, schedules,
   direct assignments, dose occurrences, takes, stock and review cases.
5. Add snapshots, change feeds, batches, offline replay, ETags, and
   idempotency cases, including revocation between original write and replay.
6. Add administrator, invitation, exports, reports/PDF, upload, notification,
   lookup, FHIR, and MCP cases where those surfaces remain part of MedTracker.

Each task updates the matrix, runs its tests against Rails, writes its exact
commands and results to a unique report, and receives independent requirements
and code-quality verdicts. A case is complete when the test proves an
observable outcome, not when it mirrors an RSpec implementation detail.

## Required assertions

Check HTTP method/path, status, content type, response shape and selected
values, error shape, auth/permission decision, and externally visible side
effects. Include failure and duplicate/retry paths for writes. Use the
existing public reads or audit interface to confirm stock, dose, access and
audit changes. Verify whole-batch rollback and that revoked access cannot
replay a cached successful response. Preserve decimal and timestamp precision.

## Stop and report

Stop a task when Rails and OpenAPI disagree, a fixture requires a production
auth bypass, an endpoint cannot be exercised without real external services,
or a case exposes a data-safety or clinical-rule ambiguity. Record the exact
route, source test, observed behaviour, proposed ruling, and cost of a wrong
decision. Continue unaffected cases while the controller resolves it.
