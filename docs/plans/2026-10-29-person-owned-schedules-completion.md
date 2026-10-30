# Person-owned schedules and care sharing — completion plan

Issue: <https://github.com/damacus/med-tracker/issues/2684> and
<https://github.com/damacus/med-tracker/issues/2976>.
Blocking bugs: <https://github.com/damacus/med-tracker/issues/2994> and
<https://github.com/damacus/med-tracker/issues/2995>.

This plan resumes the work of
`docs/plans/2026-10-28-person-owned-schedules-2684.md` (the binding
specification: ADRs 0013–0017, the spec's Global Constraints, and the settled
decisions recorded in the 29 October handoff). That plan's SDD workspace at
`.superpowers/sdd/2026-10-28-person-owned-schedules-2684/` is preserved
read-only; its ledger `progress.md` and its `Ruling:` entries remain
authoritative history.

## Resume state (verified 29 October)

- Branch `codex/person-owned-schedules-2684`, HEAD `56fb866f2`, with 68
  modified tracked + 28 untracked files of uncommitted Task 4 work.
  `.serena/project.yml` is an unowned unrelated modification: preserve it,
  never commit it. `docs/plans/*.md` stay untracked.
- Latest focused run: 18 tests, 11 pass / 7 fail / 762 skip. Five adoption
  paths return HTTP 500 `internal_error`; the injected index-failure test
  reached its witness but returned 500 instead of 503; the current-guardian
  frozen-Rx adoption request timed out (reqwest `TimedOut`) after a 204
  departure. Server-side causes were never captured; scoped review of the
  live-adoption slice never started.
- Evidence: `.superpowers/sdd/2026-10-28-person-owned-schedules-2684/evidence/2026-10-29-live-adoption/`
  holds the run log, 951-file manifest, 12-file slice diff, before-copies,
  and the prepared resume command
  `task4-live-history-eighteen-resume-command-2026-10-29.fish`.
- `live-adoption-history-runtime-2026-10-29.md` and
  `live-adoption-history-proposal-2026-10-29.md` in the same workspace describe
  the authored scope and the frozen candidate.
- iOS sibling `/Users/damacus/repos/damacus/med-tracker-ios`, branch
  `codex/person-owned-schedules-contract`, HEAD `b7523e5`, clean.

## Global Constraints

- Follow AGENTS.md and the three repository Rust skills. Fish/RTK and
  repository `task` entries only; no Python anywhere, including temporary
  scripts. Use `jq` for JSON. Write and observe a failing observable test
  before executable changes; never weaken assertions, increase timeouts, or
  serialise the suite to hide failures.
- Typed commands/results and distinct IDs; exact Decimal arithmetic; SeaORM
  entities/relations/expressions/query builders/transactions for expressible
  database work. Raw SQL only for documented unsupported RLS/trigger/DDL —
  isolated, parameterised, preserving tenant/RLS/transaction boundaries.
- Restricted runtime credentials and real PostgreSQL 18 RLS. Recheck current
  account, membership, disclosure and guardian authority at
  mutation/replay/queue boundaries, including after waits. Preserve
  audit/outbox/stock atomicity, replay protection, signed keys, ETags,
  existing local/portable IDs and HTTP/MCP envelopes.
- Existing fixture and nextest setup only: run-owned disposable
  PostgreSQL/RustFS via `compose.yaml`, template-cloned fixtures, generated
  scenario IDs, tenant-scoped assertions. No new reset framework, no global
  truncation, no populated-database adoption, no administrator runtime
  credentials.
- Shared build cache `/private/tmp/medtracker-2702-native-ci/{target,build}`:
  verify no other owner is using it before running builds/tests; never
  clean, move, or copy it; never run a second concurrent build; never run an
  old compiled binary directly.
- Preserve external worktrees, the separate checkout at
  `/Users/damacus/repos/damacus/med-tracker`, the unowned
  `.serena/project.yml` modification, and sibling `.superpowers/sdd/`
  workspaces. No production, live-data, deploy, merge, or history-rewrite
  actions. #2381 stays held.
- Settled decisions: keep the sharer's original named timezone and DST; an
  old unknown zone is adopted from the configured application zone only at
  actual authorised adoption with `adopted_application_zone` recorded. Match
  medicines via reliable existing internal IDs or complete NHS dm+d
  identifiers and exact unit — never name/strength-text inference or
  conversions. One approval per person/household covers current and future
  plans; clearly worded invitation acceptance may include it. Leaving ends
  sharing ties; rejoining needs fresh approval; suspension blocks current
  access without being departure. A current authorised guardian can approve
  and edit from another household; household manager status alone gives no
  clinical authority — managers bind exact local stock only. Share current
  care and minimal safety information; foreign detailed dose history,
  private notes and pause narratives stay private.
- Do not implement #2685 history readers, #2845 SMTP, paused #2680 optional
  locations, stock transfer, generic household navigation, or new reports.
  #2686 owns assignment/unassignment completion — preserve assignments and
  reusable interfaces without claiming it done. Coordinate sync/restore
  adapters with #2689 and portable-history with #2702 without claiming their
  scopes.
- Conventional Commits; verify author, committer and signing identity before
  every commit — if 1Password signing is unavailable, disable signing for
  that commit only. Read exact identity/signing config keys only; never dump
  Git config. No push until the publication task.
- Keep visible-UI verification through the browser at desktop/mobile sizes
  with screenshots under `docs/screenshots/`; keyboard access, labels,
  errors, focus, contrast and dialog handling accessible; never put queries
  in template loops.

## Task 1: Diagnose the adoption HTTP 500s and guardian adoption timeout

Research task — produce a diagnosis report only, no code changes. The saved
run log contains only `internal_error` request IDs; the server-side cause was
never captured.

1. Read `.superpowers/sdd/2026-10-28-person-owned-schedules-2684/live-adoption-history-runtime-2026-10-29.md`,
   the saved run log and 12-file slice diff under
   `evidence/2026-10-29-live-adoption/`, `src/models/care/person_schedules.rs`
   (`save_execution` adoption flow around the `adoption_history::capture`
   call), `src/models/care/person_schedules/adoption_history.rs`,
   `migration/adoption-history.sql`, the `shared_administration_guard`
   function in `migration/approved-sharing.sql`, the RLS policies on
   `shared_schedule_adoption_facts`, and the `OperationError` → HTTP status
   mapping.
2. Reproduce exactly one failing case —
   `person_owned_schedules::adoption_history::live_adoption_indexes_truth_before_current_dates_and_limits`
   — with server-side logging visible, through the native task entry used by
   the saved resume command (same env: shared
   `/private/tmp/medtracker-2702-native-ci` target/build dirs,
   `CARGO_BUILD_JOBS=2`, `CARGO_NET_OFFLINE=true`, `NEXTEST_TEST_THREADS=4`,
   `npm_config_offline=true`, `mise exec --no-deps node@26.12.1`,
   `task host:test:integration TARGET=care_api 'CLI_ARGS=-E "test(=…)"'`).
   Capture the actual PostgreSQL error (RLS rejection, trigger
   `RAISE EXCEPTION`, or a Rust-side error) behind the 500. First verify the
   shared cache is not in use by another owner.
3. Statically analyse the guardian timeout: the
   `legacy_departure_current_guardian_adopts_frozen_rx_without_former_source_or_stock_reader`
   path from departure (204) through the adopt request — lock ordering
   (`lock_shared`, `lock_exclusive`, trigger `FOR SHARE`/`FOR UPDATE`), the
   cross-household authority path in `capture`, and the request's transaction
   boundaries. Reproduce only if cheap; report whether it shares the 500s'
   root cause or is a distinct defect.
4. Report: root cause(s) with evidence, exact defect location(s), the minimal
   fix surface, whether the index-failure 500-vs-503 case is the same defect
   or an error-taxonomy defect, and anything the fix must NOT change. If not
   root-caused after two evidence-based attempts, report ranked hypotheses
   and what was tried — do not guess a fix.

## Task 2: Fix #2994/#2995, rerun the exact 18 tests, commit pending state and fix

1. Read the Task 1 diagnosis report (path supplied at dispatch) and confirm
   it against the code before editing. The seven failing tests are the RED
   evidence; add narrower focused tests only where the diagnosis exposes a
   gap the 18 do not cover (for example a unit-level error-mapping check).
2. Implement the minimal root-cause fix under TDD. Keep the documented
   SeaORM/RLS boundary: logic fixes belong in Rust or in the SQL-guard layer
   only where SQL is the documented unsupported-operations boundary. Do not
   weaken any assertion, raise timeouts, or bypass the guard/RLS checks —
   the fix must make legitimate adoptions pass while keeping the forgery,
   immutability, late-capture and rollback protections intact.
3. Verify with the same 18-test selection into a NEW log by running
   `rtk proxy fish .superpowers/sdd/2026-10-28-person-owned-schedules-2684/evidence/2026-10-29-live-adoption/task4-live-history-eighteen-resume-command-2026-10-29.fish`
   from the worktree root (it writes
   `/tmp/task4-live-history-eighteen-resume-2026-10-29.log`; preserve or
   rename that file before any second run). Expect 18/18 — including the
   previously unreached preservation, permission and replay assertions.
4. Only after the 18 pass, create two logical commits with verified
   identity/signing: (a) a checkpoint commit of the preserved pending state
   exactly as it stood — everything except `.serena/project.yml` and
   `docs/plans/*.md` — under a `feat(care):` Conventional Commit; (b) the
   fix under a `fix(care):` Conventional Commit. Record the exact SHAs in
   the report.
5. Report RED/GREEN commands and output, changed paths, the diagnosis-vs-fix
   mapping, and any remaining concerns.

## Task 3: Independent review of the fix and the live-adoption history slice

Review-only task (no implementation). Two inputs: the fix diff
(checkpoint..fix-head package generated by the controller) and the 12-file
live-adoption slice as it now stands after the fix (saved original diff at
`evidence/2026-10-29-live-adoption/task4-live-adoption-slice-2026-10-29.diff`).
Spec: `live-adoption-history-proposal-2026-10-29.md`, plan Task 4 items 1–8
from `docs/plans/2026-10-28-person-owned-schedules-2684.md`, and this plan's
Global Constraints. Verify: typed immutable receipts captured before source
association; minimal safety indexing/latest-time publication in the same
transaction; restricted-runtime RLS and authority rechecks; no new
grant/reader/definer; error-path classification (no raw DB details to
clients); SeaORM boundary discipline. Deliver spec-compliance and quality
verdicts with file:line findings.

## Task 4: Merge current origin/main and renumber unpublished migrations

1. `git fetch origin`; merge `origin/main` into
   `codex/person-owned-schedules-2684` as a recorded local integration
   commit (preserves history and attribution; this is not a PR merge).
2. Preserve the published migration history (published migration 19 must not
   be renumbered) and the separately-merged #2990 state. Renumber only our
   unpublished migrations (`m20261028_000021_approved_sharing`,
   `m20261029_000022_legacy_schedule_prescriptions`,
   `m20261029_000023_live_adoption_history`) into the first free slots after
   main's latest migration; update `migration/src/lib.rs` registration and
   any dependent references.
3. Resolve conflicts per the spec — keep the sharing feature's semantics
   intact; where main changed neighbouring code, integrate rather than
   revert.
4. Verify: compile, the same 18-test selection green, `git diff --check`.
   Report the merged main SHA, new migration numbers, conflict resolutions,
   and evidence.

## Task 5: Frozen-departure history, restore, and backfill

1. Earlier original doses must count toward safety limits (a) after the
   originating participation ends (frozen legacy evidence), (b) when an
   existing schedule is upgraded to a personal plan, and (c) on backup
   restore through RestoreHistory — preserving original doses, stock and
   actors with no side effects (no stock deduction, no live reminders/mail).
2. Require BOTH household import permission and current person/guardian
   authority for any history application; provide migration backfill of
   adoption facts for already-linked sources.
3. Cover forgery/idempotence/side-effect-free restoration and the unresolved
   original-concurrency paths from the earlier 13-case run's eight
   pre-assertion HTTP timeouts.
4. Coordinate portable-history behaviour with #2702 — shared adapters only,
   do not claim its scope. TDD with focused App::boot restricted-runtime
   acceptance; no broad suite replay while iterating.

## Task 6: Internal-ID compatibility evidence

Test retaining prescriptions with supported internal medicine IDs but no NHS
dm+d catalogue entry — an unresolved compatibility question, not a reproduced
bug. Prove the current behaviour with focused tests, document the observed
answer, and never invent catalogue facts or replacement-stock equivalence.

## Task 7: Browser journeys, locales, accessibility, screenshots

1. Finish the find/edit/adopt journeys in the browser for the person and for
   a current guardian in another household: consent wording, approval state,
   stock binding by manager, and denial surfaces.
2. Foreign private history and notes stay hidden; local stock selection stays
   visibly separate from clinical permission.
3. Finish translations across `assets/reports/locales/{cy,en,es,ga,pt}.yml`
   for all new copy; no untranslated keys.
4. Keep keyboard access, labels, errors, focus, contrast and dialog handling
   accessible; never put queries in template loops.
5. Verify with `rtk task test:browser` (filtered where iterating) and capture
   desktop + mobile screenshots under `docs/screenshots/`.

## Task 8: Final client generation

Freeze the final API document; regenerate backend OpenAPI canonical plus the
Android and iOS clients from the SAME final document. Preserve generator
provenance and integrity pins; keep the documented Android okhttp4/Moshi
adapter limitation and its derived input distinct from the canonical pin;
never hand-edit generated clients. iOS work commits in
`/Users/damacus/repos/damacus/med-tracker-ios` on
`codex/person-owned-schedules-contract` with verified Dan Webb identity and
signing — no push in this task.

## Task 9: Backend full verification gates

Run `rtk task ci`, `rtk task docs:build`, and `git diff --check` on the
frozen candidate; capture actual selected/pass/fail/skip counts with the
run-owned disposable foundation. Concrete failures route to a bounded fix
round — no broad replay between small fixes.

## Task 10: iOS native verification gates

In `/Users/damacus/repos/damacus/med-tracker-ios` under Xcode 27.1 / iOS 27.1:
`task ci` covering lint, unit, UI, build and the universal iPhone/iPad
release archive; iPhone visual evidence in light/dark and at an
accessibility text size; preserve prior xcresults. iPad visual verification
is owner-deferred — record it as deferred, do not downgrade targets. Use
`task lint`, `task test`, and the `api:*` entries as scoped.

## Task 11: Final whole-branch review and security review

1. Whole-branch review over the merge-base..HEAD package plus the ledger's
   deferred-minor and parked findings — spec and quality verdicts against
   the plan and Global Constraints.
2. Dedicated security review of the cross-household surface: RLS policies,
   guards/triggers, authority rechecks after waits, privacy of foreign
   history/notes/pause narratives, replay and immutability protections, and
   error-path information leakage. Findings route through one consolidated
   fix dispatch and one scoped re-review.

## Task 12: Publication and closeout

Only after every gate is green: push `codex/person-owned-schedules-2684` to
GitHub and open a PR against `main` via the github-pr skill (evidence-based
summary and test plan); push the iOS branch to its Forgejo upstream and open
its PR. Update issues #2684/#2976 checklists and #2994/#2995 with read-back
verified, plain-UK-English bodies. Report local, published, accepted and
deployed states separately; include every ledger `Ruling:` line in order
with costs; record the iPad deferral and the unverified OAuth-rotation note.
No merge, deploy, or live-data action.

## Completion gates

- Task-scoped spec + quality verdicts for every task; final whole-branch and
  security reviews clean or findings adjudicated with rulings.
- 18/18 focused tests, backend `task ci`/`docs:build`/`git diff --check`,
  iOS `task ci` and visual evidence all green on the frozen candidate.
- PRs published and linked; issue updates read-back verified. No merge,
  deployment, or physical/live acceptance claim.
