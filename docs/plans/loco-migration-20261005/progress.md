# Migration progress

Updated 5 October 2026. We are still finishing setup. Medication features have
not moved into Loco yet. No slice is accepted.

| Slice | Current state | What must happen next |
| --- | --- | --- |
| 1. Set up Loco and move Rails | The tested setup commit is pushed; Loco checks pass on the local merge with main | Finish documentation checks, publish the merge and verify GitHub CI |
| 2. Use the existing database | Plan and schema inputs prepared | Implement and verify database adoption, household access and Rails rollback after setup is accepted |
| 3. Sign-in and API authentication | Separate library experiment has 37 of 39 tests passing | Resolve the two library limitations and prove stored credentials, tokens and security behaviour |
| 4. Household and medication actions | Not migrated | Implement complete authorised care workflows |
| 5. APIs and integrations | Not migrated | Migrate the existing API, native clients and integrations |
| 6. Browser pages and themes | Not migrated | Implement complete browser workflows and port existing themes to daisyUI |
| 7. Background jobs and reminders | Not migrated | Establish durable queues and migrate jobs and schedules |
| 8. Release images and rollback | Not completed | Verify both scratch-image architectures and populated Rails rollback |

## Setup: what passed and what failed

The setup fixes passed source review, root Loco checks, both browser test runs,
Ruby lint and all 6,112 Rails examples. These fixes are committed locally as
`9e54e9305083cfa9b22708e9feba0511cd48c18d`. That exact commit has now been pushed
to the existing draft PR. The later MCP dependency merge remains local. The added
Rails diagnostic has been archived outside the repository and removed.

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
Documentation checks and publication follow.

The combined full-run log is
`/Users/damacus/Library/Application Support/rtk/tee/1791225510_task_rai_9786c9.log`.
The focused run has finished and its temporary resources have been removed.
Earlier test and review evidence is retained in [foundation-report.md](foundation-report.md).
Earlier results do not prove that the current combined code passes.

## Publication and remaining work

[Draft PR #2451](https://github.com/damacus/med-tracker/pull/2451) now contains the
setup correction at `9e54e930`. Passing hosted CI remains unverified. The later
combined code still needs its checks, commit and push.
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
