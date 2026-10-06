# Scratch runtime and wholesale replacement Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Produce the complete no-Ruby Loco deployment artefact and prove Rails rollback before cutover approval.
**Architecture:** A statically linked scratch image contains the accepted server/worker plus required runtime
assets. Remove superseded routing/tooling only after full replacement tests pass independently of legacy Rust.
**Tech Stack:** Rust/Loco, PostgreSQL 18, scratch, linux/amd64 and linux/arm64, maintained TLS/PDF/timezone libraries.
**Spec:** [implementation-spec.md](../implementation-spec.md), retained PR #2418 and release requirements in `../plan.md`.

## Global constraints

All capability owners accepted first. Never deploy the foundation as the finished app.
External production credentials, binding/host settings and connection limits are mandatory.
No live migration/deployment/merge: produce reviewable artefacts and a rehearsed cutover procedure.

## Review focus

Missing TLS CA roots (R1), missing font/timezone/template assets (R1), architecture-specific runtime
failure (R1), hidden legacy routing dependency (R2), and incompatible populated-database rollback (R3).

### R1: Build and run actual scratch server and worker on both architectures

**Files:** Create root `Dockerfile`, `scripts/migration/release-runtime.mjs`,
`scripts/migration/release-runtime.test.mjs`, `tests/release_config.rs`;
modify root config production settings, `Taskfile.yml` and relevant CI workflow.
**Interfaces:** `task release-image PLATFORM=linux/amd64|linux/arm64` builds the final artefact;
`task release:verify PLATFORM=...` runs that artefact with owned fixtures and both process modes.
Image runtime includes CA roots, licensed fonts, templates/assets, timezone capability and writable private storage.

Retained input: PR #2418 is still open at `d11483c2dcfaa5e3cdbcaaab8f3f5707512efc0b`.
Its `rust/contract-tests/Dockerfile` supplies useful musl build stages, a static-link
check, CA roots, the PDF font, writable `/tmp` and non-root scratch execution.
Adapt those parts to the root Loco binary and runtime assets. Its release Task
selects only the host architecture; it does not prove the two-platform server and
worker requirement. Close the PR only after its useful work has a verified replacement.
The maintained WebAuthn dependency adds OpenSSL. Verify static linkage of that
dependency on both architectures as part of the actual binary check; a successful
host build does not establish that the scratch image can run it.

- [ ] Test absent required production credentials/settings and final-image operations.
  Assert `missing_credentials_boots == false`, `https_request_succeeds == true`,
  `pdf_font_used == expected_font`, `dst_boundary_correct == true`, `private_storage_writable == true`.
- [ ] Register the gated Tasks/tests; run each platform's runtime test and record the current missing artefact.
- [ ] Build with the retained scratch work's useful assets and maintained static-compatible dependencies.
  Replace all development-only production defaults; distinguish server/worker health and shutdown semantics.
- [ ] Run `rtk task release:verify PLATFORM=linux/amd64` and `PLATFORM=linux/arm64` against actual
  artefacts, using a real runner or documented emulation. Manifest inspection is insufficient.
- [ ] Review and commit `feat(release): package complete Loco server and worker in scratch`.

### R2: Remove superseded code and Tasks without losing capabilities

**Files:** Modify root `Taskfile.yml`, `.github/workflows/ci.yml`, routing/module/dependency files,
active docs and `AGENTS.md`/`agents.md`; remove accepted obsolete `rust/api`, `rust/web`,
`rust/ui-preview`, Leptodon/Loom dependencies and their Taskfiles. Retain useful black-box contracts
under `tests/contracts/` with an independent manifest if needed; keep client tools independent.
**Interfaces:** Root public Tasks invoke Loco only; `rails:` and standalone Rails remain functional.
`task slice:contracts`/`slice:browser` operate without legacy source/build products.

- [ ] Test a clean checkout without legacy binaries and with only final runtime assets.
  Assert `legacy_dispatch_calls == 0`, `legacy_ui_runtime_dependencies == []`,
  `missing_capability_rows == []`, `rails_rollback_tasks_work == true`.
- [ ] Run final route/capability inventory and no-legacy acceptance; record dependency failures before removal.
- [ ] Remove only replacements already accepted; migrate retained tests before deleting their owning crate.
  Preserve comments in surviving sources; remove obsolete aliases and update active commands/guides together.
- [ ] Run `rtk task ci`, complete Loco API/browser/worker checks, native checks and `rtk task docs:build`.
- [ ] Review and commit `refactor(app): remove superseded Rust application infrastructure`;
  close remaining input PRs only with captured-source and accepted-replacement links.

### R3: Rehearse populated upgrade, rollback and complete operation acceptance

**Files:** Create `scripts/migration/cutover-rehearsal.mjs`,
`docs/operations/loco-cutover-runbook.md`, `tests/cutover.rs`; update migration progress/HTML report,
PR #2451 and issue #2450.
**Interfaces:** New `task release:rehearse` creates representative populated synthetic data, adopts
the schema, exercises full Loco journeys, runs Rails rollback and verifies preserved state.

- [ ] Test whole-dataset identity/credential/audit preservation, real native sign-in/dose flow,
  offline replay, worker restart, cross-household denial and rollback after accepted writes.
  Assert `before_ids == after_ids`, `successful_audits_preserved == true`,
  `duplicate_doses == 0`, `rails_rollback_journeys_succeed == true`.
- [ ] Rehearse the approved unsupported-passkey transition: identify affected accounts,
  verify clear replacement instructions, supported-key use on mixed accounts and
  recovery/re-enrolment for unsupported-only accounts without silently bypassing MFA.
  Preserve unsupported credential rows and prove they remain available to Rails rollback.
  PS256 authentication in Loco is deliberately excluded by the 6 October decision;
  the user recovery/replacement journey is a required cutover check.
- [ ] Run `rtk task release:rehearse`; record unimplemented rehearsal failures.
- [ ] Implement the disposable rehearsal and operational cutover/rollback instructions with explicit
  image identity, schema checks, storage/queue compatibility and go/no-go evidence.
- [ ] Run both-platform runtime, complete journeys/rehearsal and final broad independent review on one
  frozen commit; require green hosted CI and all capability rows accepted.
- [ ] Commit, pull with rebase and push; report published artefacts, review verdict and remaining approval
  boundary. Request deployment/cutover permission only with the concrete complete result available.

**Done:** Complete Loco capability parity, both scratch runtimes, no hidden legacy application dependency,
populated rollback and clean reviewed/published evidence. Deployment itself remains separately authorised.
