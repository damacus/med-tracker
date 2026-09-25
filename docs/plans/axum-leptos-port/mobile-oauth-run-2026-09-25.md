# Mobile OAuth medication read tranche

Baseline: `7bec286a`. Charter: [team-charter.md](team-charter.md).
Status: approved for execution by the user's continuing port instruction.

## Outcome

Existing mobile OAuth grants authenticate medication list/show requests using
current account and household authority. Invalid grants cannot expose data.
This does not implement OAuth issuance, refresh endpoints, integration grants,
app tokens, medication writes, or broader browser/PWA parity.

## Test writer brief

Own `rust/contract-tests/tests/medication_mobile_oauth_api.rs`,
`rust/contract-tests/src/lib.rs`, `scripts/contract_provision.rb`, and
`Taskfiles/contract.yml`. Own report `mobile-oauth-tests-report.md` here.
Read Rails OAuth grant/authentication code and request specs to derive the
contract. Prefer existing fixtures; add isolated fixtures only when needed.
Wire the new target into the existing medication acceptance task without
editing run.fish. First establish red with a valid mobile grant against the
unchanged Rust service. Notify the orchestrator and product writer with the
exact failure before proceeding to denied cases.

Cover valid list/show, current delegated visibility, foreign-household denial,
revoked/expired grant, inactivity and maximum login age, required medtracker
scope, inactive/locked account, and revoked membership where the slice can
express these. Inspect the exact Rails contract; flag ambiguity instead of
inventing statuses. Check audit attribution identifies OAuth, does not pretend
to be ApiSession, and excludes bearer material. Verify last-used behaviour
where publicly observable or by narrowly scoped test database inspection.

## Product writer brief

Own `rust/api/src/**`, `rust/api/Cargo.toml`, and `rust/api/Cargo.lock`.
Own report `mobile-oauth-product-report.md` here. Do read-only investigation
while the first test is prepared. No production edits until its red result.
Use SeaORM and preserve restricted-role transactions, tenant GUC boundaries,
account checks, active current membership lookup, and medication visibility.
Support mobile OAuth grant digest, expiry/revocation, scope and login lifetime
rules, current user/account checks, last-used refresh, and correct audit
identity. Keep ApiSession behaviour intact. No auth bypass or raw bearer logs.
Fetch Context7 only for library usage details actually needed.

## Runner/scout brief

Own report `mobile-oauth-runner-report.md` here, no source files. Establish
exact task commands and runner constraints first, then run requested checks.
Record stable-input evidence and counts. Do not diagnose by changing tests,
code, fixtures, or gates. Also identify the smallest safe way to remove the
fixed-port bottleneck; report it as follow-up, not implementation in this slice.

## Checks and review

Focused red/green and combined HTTP: `task api:acceptance` (disposable PG18).
Static/unit: `task api:test`, `task api:fmt`, `task api:clippy`,
`task contract:fmt`, `task contract:clippy`. Fixture edits require
`task rubocop`. Documentation: `task docs:build` and `git diff --check`.
Do not run the full Rails suite for fixture-only or Rust changes.

Writers announce freezes for relevant files; the runner captures HEAD and
SHA-256 hashes of relevant tracked and new source files before/after evidence
runs, excluding generated artifacts and reports. Only one
port-39998 acceptance run at a time. Independent Sol review owns
`mobile-oauth-review.md`, with requirements and code-quality verdicts against
the full combined diff and evidence. Orchestrator updates README/matrix and
ledger after review, then commits and pushes applicable verified changes.

Ledger states: assigned, red proven, implementation, green verified, review
findings, accepted. Completion requires evidence and clean independent review.
Ledger append under migration-defined triggers remains a separate open task.
