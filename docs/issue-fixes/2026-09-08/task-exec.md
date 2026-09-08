# Issue #2124: task command execution

`test:exec` prepared Tailwind assets but, with the inherited `run: once` policy,
did not execute a distinct supplied `CMD`. The red regression copied the real
repository Taskfile graph and replaced only the compose-lock boundary with a
temporary executable stub. The success invocation returned zero and recorded
only `rails tailwindcss:build`; the supplied command was absent.

The fix keeps the root Taskfile's `run: once` policy and sets
`Taskfiles/internal.yml`'s `internal:run` task to `run: when_changed`. Distinct
environment, service, command, and Docker-argument values now run while
identical calls remain deduplicated. The README wording and `ci:check` now
document and execute the regression harness.

The green harness ran both supplied-command cases through the same copied graph:

- Success returned status 0 and recorded, in order, `rails tailwindcss:build`
  and `TASK_EXEC_REGRESSION_SUCCESS`.
- Failure returned non-zero and recorded, in order, `rails tailwindcss:build`
  and `TASK_EXEC_REGRESSION_FAILURE`.

The harness uses Node built-ins and a temporary boundary stub; it does not run
Docker or destructive commands. `node --check scripts/ci/task_exec_regression.mjs`
and `git diff --check` also passed.
