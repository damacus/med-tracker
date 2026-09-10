# Independent review: issues #2124 and #2125

## Requirements verdict

Accepted, subject to the coordinator-owned full Rails suite, Ruby lint and
documentation checks.

Issue #2124 is fixed at the shared container runner without changing the root
Taskfile policy. `internal:run` now uses `run: when_changed`, so calls with a
different environment, service, command or Docker argument execute while an
identical call can still be deduplicated. This is narrower than changing the
root `run: once` policy or making every call run unconditionally. The reviewed
call sites show the intended effect: both commands in `test:exec`, both commands
in `test:assets-rebuild`, and all three commands in `dev:assets-rebuild` can run;
single-call users keep their previous behaviour.

The regression executes the repository's real `test:exec` task graph. It copies
the Taskfiles into a temporary directory and replaces only the external
compose-lock boundary, so it can prove Tailwind preparation and the supplied
command ran in order without starting Docker. Separate success and failure
cases verify a successful supplied command and propagation of a failure from
the second command. The permanent CI check uses Node built-ins and the tools
already installed by the policy job.

Issue #2125 changes only the source rows used by the three routine headline
metrics. Next Due, Due Now and Tasks Left read `routine_tasks_by_person`;
as-needed rows continue through their existing disclosure, status, cooldown and
recording paths. Presenter coverage includes routine-only, as-needed-only,
mixed available, mixed cooldown and terminal-row cases. The browser regression
records the routine dose, reaches `None today`, `0` and `0`, then opens the As
needed section and confirms its take control remains available without an
as-needed dose being recorded.

The accepted desktop and freshly reloaded 390px screenshots show completed
routine work and the expanded As needed control without clipping. The earlier
unsettled resize capture was discarded. Canary was not used.

## Code-quality verdict

Accepted. No concrete findings remain.

The task fix gives the exceptional shared runner an explicit policy and updates
the adjacent documentation. The harness reports process-start failures clearly,
checks exact ordered boundary calls, cleans its temporary graph in `finally`,
and exercises both exit paths. Actual Docker success and failure checks close
the boundary left intentionally fake in CI.

The dashboard implementation is a small, named `routine_action_rows` extraction
used only by the existing headline calculations. It does not alter
`FamilyDashboard::ScheduleQuery`, due-now classification, terminal statuses,
cooldown calculation, dose guards, inventory changes, Turbo refresh behaviour,
or the rendered as-needed collection. The tests therefore exercise the changed
semantic boundary without duplicating dose rules.

## Verification evidence

- Reviewed the complete branch against `origin/main`, including commit
  `2653eade` and the frozen dashboard diff.
- `rtk node --check scripts/ci/task_exec_regression.mjs` passed.
- `rtk node scripts/ci/task_exec_regression.mjs` passed, proving preparation,
  supplied-command success and supplied-command failure propagation.
- `rtk git diff origin/main --check` passed.
- Actual Docker `test:exec` checks passed for `ruby -v` after Tailwind and for a
  Ruby exit 23 failure; Task returned nonzero for the latter.
- `task ci:check` passed with the documented isolated local tool path.
- Presenter regression: 33 examples, 0 failures.
- Dashboard browser regression: 6 examples, 0 failures.
- The reviewer inspected both accepted local screenshots.
- The full Rails suite and Ruby lint were still running when this review was
  completed; their results belong in `verification.md`.
