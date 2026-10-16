# Recovery of superseded pull requests

## Overview

These complete source patches preserve work from superseded PRs before closure.
They are reference material, not installed code or passing acceptance evidence.
They deliberately live on the recovery branch rather than being merged into the
application. Existing source branches are retained as a second recovery route.

Verify the compressed patch hashes with `shasum -a 256 -c docs/pr-recovery/SHA256SUMS`.
The patches contain the original PR delta; inspect before applying to a new base.

## PR 2630: combined expiry workers

Source head: `49814f580c0844e4065f3b7a083864e41b1f8722`.
Preserved source: [complete patch](2630-expiry-workers.patch.gz).
Continuation issue: [2720](https://github.com/damacus/med-tracker/issues/2720).

The combined PR overlaps three more focused deliveries. Continue through
[support expiry 2626](https://github.com/damacus/med-tracker/pull/2626),
[canary 2627](https://github.com/damacus/med-tracker/pull/2627) and
[export expiry 2629](https://github.com/damacus/med-tracker/pull/2629).
Do not merge the combined implementation over that stack.

Before treating its work as fully superseded, compare and retain these cases:

- Atomic support expiry and attributable audit, retry idempotence, and continued
  processing when another row fails. Resolve the hourly versus per-minute
  descriptions against the maintained runbook and selected implementation.
- Export retention configuration, active retention holds rechecked under lock,
  stalled generation with an attached artifact, missing artifacts, physical
  deletion, mirror failures and retry behaviour. Resolve daily versus hourly
  scheduling and the treatment of downloaded exports explicitly.
- The canary's application/job signals, correlation, span and counter tests.
  Keep operator-invoked post-deployment checks distinct from scheduled emission;
  neither a no-op global metrics provider nor unit tests proves deployed intake.

Use current Loco settings and storage ownership. Do not transplant the old
ArtifactStores map, schema joins or raw audit attribution blindly. Do not claim
that filesystem/object-store deletion rolls back with PostgreSQL: test retries
after partial physical deletion and the actual transaction/audit failure cases.
Run each selected worker's registration, denied access, retry and failure tests
on the current household schema before acceptance. This archive has not ported
or verified any worker implementation.

## PR 2628: Loco references and departure documentation

Source head: `7be3c40f88c62d77383b5989b0fa14b113e9c579`.
Preserved source: [complete patch](2628-loco-guidance.patch.gz).
Continuation issue: [2592](https://github.com/damacus/med-tracker/issues/2592).

[Merged PR 2734](https://github.com/damacus/med-tracker/pull/2734) supplies the
current Rust/household guidance. Do not restore the duplicate lowercase agent
guide, removed Rodauth compatibility advice or old schema assumptions.

The unique material to assess is the pinned Loco 1.2 reading set and recipes:
endpoint, migration/model, testing, tasks/schedules, workers, storage-related
conventions, middleware, authentication and mail. Preserve useful upstream links
and the concrete reasons for deliberate departures, with current implementation
references. Upstream source remains reference material; reusable skill changes
must follow the current repository's skill-source policy.

Reconcile each departure against current code, retain unresolved items in the
existing Loco audit issue, and build the resulting documentation. Do not mark the
audit complete merely because the obsolete documentation PR is closed.

## PR 2733: client quality gates carried forward

Source head: `8e84c685a130afe3daefaa854220e4289f0cd3a8`.
Destination: [active PR 2735](https://github.com/damacus/med-tracker/pull/2735),
published checkpoint `b2a9bb34b850becde4092c7c962d20f7f9e5214f`.

A direct Git tree comparison found no differences in the entire `client-tools/`
tree, `deny.toml`, `scripts/ci/policy.json` or
`scripts/ci/tests/client_tools_gate.test.mjs`. The Mise differences are older
OpenSpec/root runtime configuration, not additional client quality behaviour.
Keep the newer root configuration and installer correction in the active PR.

The commit histories differ, so ancestry alone did not prove duplication.
The content comparison establishes that the client implementation has already
moved to the active delivery. Its full client-tools gate also passed in the
isolated household repair checkout. This does not make the entire household PR
accepted. Retain the original branch and commit for provenance; do not merge the
older shared configuration back over the active work.
