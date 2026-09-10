# Independent review: five test reliability issues

Reviewer: Hubble on Luna high. The review was read-only.

## Requirements verdicts

- #2095: pass. The test opens the real trigger, waits for the visible listbox, and keeps the member and owner assertions.
- #2099: mostly pass. The first direct-dose interaction is correctly scoped, but a second interaction still uses a broad form lookup.
- #2140: pass. The test waits for the visible medication listbox and keeps the blocked-next-step assertion.
- #2212: pass. The test ties readiness to the trigger-owned dialog and keeps Escape, focus, and recording coverage.
- #2145: not accepted. The current verifier does not yet prove the same Playwright path used by Capybara.

## Important findings

1. The new dependency verifier is absent from the CI changed-file policy. The branch would fail change classification.
2. The Taskfile spec checks verifier source text but does not execute stale, current, refresh-failure, or launch-failure cases.
3. The repository-wide suite had one failure before the final historical-dialog change and was not rerun afterwards.
4. The verifier launches Node Playwright, while the reported browser failures use the Capybara Playwright driver. The final
   check must exercise the real Capybara path. The repository's existing Docker compatibility links are not part of this
   change and must not be described as removed.

## Minor finding

The later direct-dose interaction in `spec/system/dashboard_spec.rb` should use the same open-dialog and visible-form
scoping as the first interaction.

## Whole-tranche verdict

Do not accept. Return all findings to the same Nightingale, rerun affected checks and the full suite from the corrected
head, then obtain independent re-review.

## Re-review

Hubble confirmed that all Critical and Important findings are resolved. The refresh-failure harness now preserves
and checks the stale marker. The final suite passed with 6,231 examples, zero failures, and one pending example.

Final whole-tranche verdict: accepted.
