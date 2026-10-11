# Handoff: contract-test stack landed on main (2026-10-11)

## What shipped

A 10-PR stacked-PR chain (#2489, #2506, #2508, #2514–#2520) merged atomically
onto `main` via GitHub's async stack-merge API. Content:

- Platform support feature groundwork and CI diagnostic retention (#2489).
- 404-vs-403 fixes: `begin_household`-scoped reads now return `NotFound` for
  out-of-scope profile/notification-preference reads, while recheck failures
  stay `Forbidden` (#2506, #2508).
- Contract-test helper hardening plus `get*`/`list*` response-shape coverage
  for every routed operation (#2514–#2520).
- `listPeople` rejects `per_page` outside `1..=100` with 422 (was silently
  clamped) and documents `404`/`422` in `docs/api/openapi.v1.yaml`. The
  clamp-vs-reject difference vs Rails is an **accepted parity gap** — the
  OpenAPI contract is authoritative.
- `tests/care_api.rs` gained `mod person_carers;` (carer API tests restored).

Two side-chain PRs (#2510 location tests, #2512 schedule tests) closed as
already-landed — the stack merge had carried their content; resolved heads
were tree-identical to `main`.

## Known leftovers

- #2515's squash commit kept a `test(api):` subject — its title missed the
  `feat(api):` retitle before landing. Cosmetic, in history now.
- Seven old `git stash` entries (Oct 6–9, platform/auth preservation
  snapshots) still in the stash list — not part of this work; drop when safe.
- `rails/` pagination still clamps page/per_page where the Loco contract
  rejects — the deliberate parity gap above.
- Flaky tests seen twice under `task ci`: `Sqlx(PoolTimedOut)` in
  `assignments::assignment_update_replay_stale_and_record_only_permission`,
  and a Playwright session-cookie check in `identity-onboarding.spec.mjs`.
  Both pass on re-run; worth a flake fix eventually.

## GitHub stacked-PR API guide (next agent)

This repo has **GitHub Stacks** enabled. `gh pr merge --auto` does NOT work —
auto-merge is unsupported on stacked PRs, and on a mergeable PR the flag can
fire an immediate mid-stack merge you didn't intend. Use the REST API.

### Inspect stack membership

```fish
# PR's stack object: id, position, size, base
gh api repos/damacus/med-tracker/pulls/PR_NUMBER --jq '.stack'

# Full membership, bottom to top
gh api repos/damacus/med-tracker/stacks --jq '.[] | .pull_requests[].number'
```

### Merge (bottom-up only)

Merging the top PR merges every PR below it atomically onto the stack base.
You cannot merge a mid-stack PR alone.

```fish
# Enqueue — returns a uuid
gh api -X PUT repos/damacus/med-tracker/pulls/TOP_PR/merge-async \
  -f merge_method=squash

# Poll until status != pending (merged / failed / queued)
gh api repos/damacus/med-tracker/pulls/TOP_PR/merge-async/UUID \
  --jq '.status + " " + (.details.message // "")'
```

### Preconditions that actually bit us

1. **Linear-descendant rule**: each stacked head must contain the head below
   it as a git ancestor. If a lower PR's branch gained a commit after the
   upper PR last updated (e.g. a mid-stack merge), the stack merge fails with
   `not a linear descendant`. Fix: `gh pr update-branch N` (or merge base into
   head locally) bottom-up through the stack, then retry.
2. **Checks are evaluated at merge execution**, not enqueue. Wait for every
   stack member's required checks to pass first — a pending required check
   fails the whole atomic merge.
3. **`gh pr update-branch` is conflict-safe**: it reports
   `Cannot update PR branch due to conflicts`; resolve locally, push, retry.
4. `CONFLICTING`/`UNKNOWN` mergeable states are frequently stale during
   base moves — reproduce the merge locally before believing them. Enable
   `git config rerere.enabled true`; identical test-file conflicts recur at
   every level of the stack and rerere replays them.
5. `mergeStateStatus`: `CLEAN` = mergeable; `BLOCKED` = required check
   pending/failed; `UNSTABLE` = mergeable with non-required checks pending.
   `gh pr view N --json mergeable,mergeStateStatus,statusCheckRollup`.
6. Head branches auto-delete on squash merge; child PRs retarget to the
   deleted head's base automatically. A merged base branch may survive if an
   open PR still targets it — retarget to `main` (`gh pr edit N --base main`)
   and verify the resulting diff is non-empty before merging; if the head is
   tree-identical to main (`git diff origin/main` empty after resolving), the
   content already landed — close the PR instead of landing an empty commit.
7. In-progress job logs are unreadable (`--log` and the jobs-logs API 404);
   only completed jobs stream. Poll PR state instead.
