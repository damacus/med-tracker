# Medication journey: secure entry

Baseline `0b6fa2a8`; objective [goal.md](goal.md); ownership
[team-charter.md](team-charter.md). User authorized execution on 25 September.

## Outcome and scope

A supported user signs in through the real CSRF-protected login page,
authorizes a registered mobile client, exchanges an S256 PKCE code, chooses
an operational household, and reads its medication using the issued token.
Refresh rotation and revocation preserve the current Rails public contract.
This is the entry portion of the dose/stock/history milestone, not completion
of that whole journey or all authentication parity.

Reuse the six existing `rust/contract-tests/tests/oauth.rs` cases and add only
missing household/security assertions needed for this path. Preserve account,
lockout, scope, redirect registration, CSRF, consent, PKCE, expiry and reuse
boundaries. A password check must never bypass a required MFA/passkey step.
If a credential flow remains unsupported, reject it safely and document that
gap rather than granting access or calling it parity.

## Parallel ownership

- Test writer (Sol): existing/new focused OAuth/household HTTP tests,
  `rust/contract-tests/src/lib.rs`, `scripts/contract_provision.rb`,
  `Taskfiles/contract.yml`, canonical `run_medication_api_tests.fish`, and
  `rust/web/tests/**`. Own `journey-auth-tests-report.md` here.
- Product writer (Sol): `rust/api/src/**`, API Cargo manifest/lockfile,
  `rust/web/src/**`, web Cargo manifest/lockfile. Own
  `journey-auth-product-report.md`. Read-only design before proven red; then
  implementation. Propose browser-session and OAuth transaction boundaries
  to orchestrator/reviewer before introducing new shared abstractions.
- Runner (Luna): checks and `journey-auth-runner-report.md` only.
- Independent reviewer (Sol): requirements/security/quality review and
  `journey-auth-review.md` only; start with the security boundary design.
- Orchestrator: goal/plan/ledger/README, necessary Docker build wiring after
  explicit handoff, integration, acceptance and Git publication.

Share the checkout with disjoint files. No new worktree is needed. No agent
commits, rebases, pushes, changes unrelated files or adds/removes comments.

## Sequence and evidence

1. Test writer wires existing OAuth contracts into the current isolated
   runner and demonstrates a meaningful Rust red. Existing Rails green
   evidence remains the baseline unless a specific ambiguity needs probing.
2. Product writer implements while test writer adds denial/browser cases.
   Reuse existing fixtures and runner; no runner redesign or broad inventory.
3. Obtain a stable combined HTTP green including prior medication regression
   cases. Review code concurrently; fix findings with the owning writer.
4. Verify the functional login/consent UI in an actual browser, including
   keyboard/form errors and desktop/mobile screenshots for visible changes.
   Do not claim a complete web medication journey from OAuth HTTP tests.
5. Record reviewed acceptance and continue toward dose creation/history.

Use `task api:acceptance` with a checked free `CONTRACT_TEST_SUBNET` on this
host. Internal Compose networking is already implemented. Run one acceptance
instance unless independent work actually benefits from a second. Capture
the complete tracked/new runtime input set or immutable image evidence, plus
fixture identity and exact results; do not invent proof from partial hashes.

Other gates: `task api:test`, `task api:fmt`, `task api:clippy`,
`task contract:fmt`, `task contract:clippy`, `task rubocop` for fixture edits,
web crate Task checks for web changes, `task docs:build`, `git diff --check`.
No full Rails suite for Rust/test-fixture-only changes. Do not repeat passing
checks without changed inputs or a concrete remaining risk.

No live data, deployment, merge, external messages, auth bypass, unbounded
session storage, or bearer/credential logging. Preserve restricted-role
database access and transaction-local tenant settings. Escalate ambiguity
or two unsuccessful fixes before broadening scope.
