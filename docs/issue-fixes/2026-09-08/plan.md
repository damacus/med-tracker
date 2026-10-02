# Task execution and routine dashboard follow-ups

Fix issues #2124 and #2125 separately from the open UI-sweep PR. This branch
starts from current origin/main. Each fix should have its own focused commit.

## Ownership

Nightingale (Luna) is the sole implementation and test writer. Root owns scope,
verification and publication. Sol independently reviews both fixes; findings
return to the same writer. Existing user approval covers implementation.

## Task execution (#2124)

Reproduce task deduplication with a failing regression before changing task
configuration. Prove supplied commands actually execute and failures propagate;
avoid merely asserting a YAML string. Keep the change narrow and preserve
container execution and asset preparation. Review other callers of any shared
run-policy change. Do not execute destructive commands in the regression.

## Routine dashboard counts (#2125)

Next Due, Due Now and Tasks Left describe routine obligations only. As-needed
availability and cooldowns stay in their own section and retain all recording
actions and dose rules. Cover as-needed-only, routine-only and mixed rows,
including as-needed cooldown. Add a real browser regression showing routine
work complete with the medicine still available in As needed. Capture local
desktop and mobile evidence. Do not use Canary.

## Verification and delivery

Use repository task commands and red-green tests. Rails preflight precedes
dashboard production changes. Run focused checks, independent whole-change
review, full Rails suite and Ruby lint before pushing. Root publishes a separate
PR with both issue references and actual local verification limits.

## Progress

- Both issue descriptions inspected; branch isolated from the UI sweep.
- Rails preflight passed: 31 examples, no failures.
- Independent design review selected a per-task `when_changed` override for
  `internal:run`, preserving the global policy and deduplication of identical
  commands. Regression exercises actual Taskfiles with a fake container boundary.
- Both fixes implemented with red-green regressions; the task-runner fix is
  committed separately as `2653eade`.
- Final independent review accepted with no findings. Full Rails suite, Ruby
  lint, documentation build and whitespace checks passed; see `verification.md`
  for exact results and the local optional-linter integration limitation.
