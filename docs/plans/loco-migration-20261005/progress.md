# Migration progress

Updated 5 October 2026. We are still finishing setup. Medication features have
not moved into Loco yet. No slice is accepted.

| Slice | Current state | What must happen next |
| --- | --- | --- |
| 1. Set up Loco and move Rails | Setup and merges with main are published; two GitHub setup failures have local fixes | Verify container permissions, finish review, publish corrections and pass hosted CI |
| 2. Use the existing database | Isolated Rust test runner passes real PostgreSQL success and failure checks locally | Finish review and publish the runner; adopt the schema after setup is accepted |
| 3. Sign-in and API authentication | Separate library experiment has 37 of 39 tests passing | Resolve two proof failures and verify stored credentials, tokens and security behaviour |
| 4. Household and medication actions | Not migrated | Implement complete authorised care workflows |
| 5. APIs and integrations | Not migrated | Migrate the existing API, native clients and integrations |
| 6. Browser pages and themes | Not migrated | Implement complete browser workflows and port existing themes to daisyUI |
| 7. Background jobs and reminders | Not migrated | Establish durable queues and migrate jobs and schedules |
| 8. Release images and rollback | Not completed | Verify both scratch-image architectures and populated Rails rollback |

## Setup: what passed and what failed

The setup fixes passed source review, root Loco checks, both browser test runs,
Ruby lint and all 6,112 Rails examples. Setup and both upstream dependency merges
are published as `9e499003f9b321a355a133c6a976f42278ab836f` in the draft PR.
The added Rails diagnostic has been archived outside the repository and removed.

Main's MCP dependency update was then included in a normal merge. Testing the
combined code produced one failure in 6,112 examples: scanning medicine to add
stock did not show the expected success message. The screenshot showed 30 units;
the database assertion was not reached. All eight examples in that test file
passed when run separately; a later focused run reproduced the original failure.
A separate diagnostic passed, but did not control cookie-application timing and
could not prove the cause. Framework inspection showed how a read-only lookup
can write an older session cookie and lose the POST's notice. We have not shown
that the migration caused this behaviour.

The user redirected the work away from repairing legacy RSpec tests. The original
Rails application and tests remain unchanged. The added diagnostic was archived
and removed. Loco checks and migrated Rust/browser workflows drive implementation;
the existing Rails rollback evidence remains part of final release verification.
No application fix or claim of a passing later Rails run was made.

Loco `task ci` passes on the combined source. Inventory and preservation checks
also pass. Preservation records 2,933 inputs, with 2,883 unchanged and 50 changed;
the changes include upstream's dependency update. The current Loco CI log is
`/Users/damacus/Library/Application Support/rtk/tee/1791227912_task_ci.log`.
Documentation and client-tool checks also passed before publication.

GitHub run `37363675318` finished with setup failures. The workflow checker lacked
Fish. Rust browser fixtures could not write `/app/tmp/local_secret.txt`. A clean
checkout lacks the Rails temporary directory; the contract runner now creates it
before containers start. Both changes have failing regressions followed by passing
checks. All 91 workflow tests pass locally. Real container permission verification
and independent review remain pending. The hosted Loco job was cancelled without
running, so it supplies no foundation acceptance evidence.

The new isolated Rust database runner ignores ambient database addresses and
Compose overrides, preserves unit and documentation tests, and cleans only its
own fixture. Four runner checks pass. Real PostgreSQL 18 success and deliberate
failure runs both removed all their owned resources. Rust formatting and lint
pass. Devin SWE-2 Max completed review, and its three requested corrections
passed focused re-review. Full local Loco CI passes on the final source, including
the real PostgreSQL integration and HTTP health/Tera checks. The changes are being
prepared for commit and push; corrected hosted Linux checks remain required.
The exact preserved SQL baseline is pinned; no schema has been adopted.

The combined full-run log is
`/Users/damacus/Library/Application Support/rtk/tee/1791225510_task_rai_9786c9.log`.
The focused run has finished and its temporary resources have been removed.
Earlier test and review evidence is retained in [foundation-report.md](foundation-report.md).
Earlier results do not prove that the current combined code passes.

## Publication and remaining work

[Draft PR #2451](https://github.com/damacus/med-tracker/pull/2451) now contains the
setup and dependency merges at `9e499003`. Passing hosted CI remains unverified.
The runner and new setup corrections still need final verification, review,
commit and push.
The user has authorised landing verified work on main through one branch and
one PR. This does not authorise a production deployment or database change.

[Issue #2450](https://github.com/damacus/med-tracker/issues/2450) covers the whole
migration. PRs #2397, #2399, #2402 and #2403 were closed as superseded framework
choices; their captured source and branches remain available. Schema, scratch,
profile, notification and font PRs remain open until their useful work is
migrated. PR #2381 is unchanged.

The separate test-runner isolation incident remains tracked in
[#2453](https://github.com/damacus/med-tracker/issues/2453). Existing test volumes
were preserved. The database effect of the unintended invocation is unproved.

## Reporting and workflow

Update `/private/tmp/medtracker-migration-status.html` and
`/private/tmp/medtracker-2450-review-status.md` when a slice finishes or a problem
changes the next action. Say what is finished, what is being checked, what is
wrong and what happens next. Keep detailed job handles in the separate technical
checkpoint. Review the workflow every two hours and apply supported improvements.

The existing writer retains application and test ownership. The verifier owns
builds, tests and temporary resources. The coordinator owns acceptance and
publication. Independent Devin SWE-2 Max review sharing is authorised through
16 October 2026. The goal remains completion of all eight slices.
