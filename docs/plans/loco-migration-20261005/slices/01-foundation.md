# Loco foundation completion Implementation Plan

> For agentic workers: use the existing persistent team and Devin review defined in `../team-charter.md`.

**Goal:** Publish and accept a runnable Loco root with independently runnable Rails rollback tooling.
**Architecture:** Retain standard Loco boot/routes and the relocated Rails source. Correct the remaining
contract-image input failure, then review the immutable corrected snapshot.
**Tech Stack:** Loco 1.2.0, Rust, Node, Task, Docker, PostgreSQL 18, Rails.
**Spec:** [implementation-spec.md](../implementation-spec.md).

## Global constraints

All specification constraints apply. No product migration or production readiness claim in this slice.
Source writer: Nightingale; runtime owner: verifier; integration and planning: coordinator.
Read-only inputs: existing foundation review, reports, source inventory and retained PR snapshots.

## Review focus

Missing image inputs must fail before long builds (F1). Public root and standalone Tasks must reach
the same owned fixture (F2). Cleanup must preserve existing volumes (F2). Stale ledgers must fail
checks (F2). Review and CI results must identify the same published commit (F3).

### F1: Make the contract image's font input available

**Files:** Modify `rust/contract-tests/Dockerfile` and the actual owning build-context filter if needed;
test `scripts/ci/tests/loco_paths.test.mjs` and `rust/contract-tests/browser_context_test.fish`.
Read `rails/vendor/fonts/NotoSans-Regular.ttf`, `rails/vendor/fonts/OFL-1.1.txt` and runner Compose.
**Interfaces:** Consume the composed repository build context; produce the existing
`/src/rails/vendor/fonts/NotoSans-Regular.ttf` runtime path with its licence.

- [ ] Add `contract_image_includes_font_and_licence`: inspect the effective build input, not only
  host existence; assert `font_present == true` and `licence_present == true`.
- [ ] Run `rtk task api:contract-browser-context-test`; demonstrate the missing effective input with
  a scratch COPY/export probe, the real repository context and a byte-identical Dockerfile-specific
  ignore file. Keep `ci:check` Docker-free; it validates the script wiring. The existing real image
  build failure is additional RED evidence, not a reason to skip the regression.
- [ ] Correct only the owning filter/COPY path. Do not assume the reported failure proves a wrong
  Compose directory: the font exists on the host, and the filter must also be examined.
- [ ] Run `rtk task api:contract-browser-context-test`, `rtk task ci:check` and the root runner in F2;
  require successful image creation.
- [ ] Review and commit `fix(contracts): include relocated font assets in runner image` with explicit paths.

### F2: Verify both supported execution entry points

**Files:** Test existing `rust/contract-tests/run.fish`, isolation/cleanup regressions;
generate `../source-inventory.json` and `../preservation.json` after source freeze.
**Interfaces:** Consume F1 image; produce complete root/standalone runner logs and owned-resource cleanup.

- [ ] Add a failing regression for any new ownership/path defect discovered by F1; preserve existing
  negative isolation checks. Assert `foreign_volumes_removed == 0` on failure cleanup.
- [ ] Run `rtk task ci:check` for any added regression and record its actual failing assertion.
- [ ] Make only demonstrated repairs through the same writer; preserve public wrapper ownership.
- [ ] Freeze source; run `rtk task inventory:write`, `rtk task preservation:write`, `rtk task ci`,
  `rtk task api:browser-dashboard-rust`, and `rtk task --dir rust/api browser-dashboard-rust`.
  Require both complete runner verdicts and zero remaining owned resources; do not credit Tailwind alone.
- [ ] Integrate the verified code and generated records with an atomic Conventional Commit.

### F3: Accept and publish the corrected foundation

**Files:** Update `../foundation-report.md`, `../foundation-review-20261005.md`, `../progress.md`
and the HTML/Markdown progress reports; update PR #2451 with the final scope.
**Interfaces:** Consume F2 immutable commit and evidence; produce an accepted published foundation.

- [ ] Run `rtk task docs:build` and `rtk proxy git diff --check`.
- [ ] Send the corrected exact diff and evidence to Devin CLI `swe-2-max`; disable textconv/external
  diff and exclude secrets. Existing sharing approval lasts through 16 October 2026.
- [ ] Resolve actionable findings, rerun affected checks and obtain clean re-review.
- [ ] Commit, pull with rebase and push after all applicable gates; verify hosted CI on the pushed head.
- [ ] Update both progress reports: shipped commit, checks, clean verdict, remaining product slices.

**Done:** Both runners complete, root CI and documentation checks pass, review is clean and hosted CI
is green on the corrected published commit. Foundation remains a pre-cutover waypoint.
