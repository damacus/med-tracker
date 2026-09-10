# Nightingale brief: five test reliability issues

Implement the approved tranche in `plan.md`. Preserve unrelated work and use only the owned paths.

## Required behaviours

1. #2095 selects the visible Member option only after the membership-role combobox is open. Keep the member and
   owner result assertions.
2. #2140 selects Dose-less medication only after the medication combobox is open. Keep the blocked-next-step
   behaviour.
3. #2212 opens the historical-dose dialog through the real trigger and waits for the open dialog. Keep Escape,
   focus, and dose-recording coverage.
4. #2099 opens the direct-medication dialog through the real trigger and waits for the visible form. Keep the
   all-family selection and dose-recording behaviour.
5. #2145 makes test preflight detect a stale mounted `node_modules`, refresh it from the current lockfile, and prove
   that the installed Chromium can launch. Do not use browser-revision symlinks.

## TDD and report

For each issue, first add or tighten a regression that fails against the current behaviour. Then make the smallest
fix. Run focused checks before broad checks. Write `report.md` with the red evidence, changed files, exact commands
and results, self-review, concerns, and next safe action. Return only a compact summary to Bucky.
