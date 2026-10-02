# Test maintenance cleanup

Remove tests that only freeze incidental decoration, repeat stronger checks, or
retest Ruby and Rails defaults. Preserve application behaviour and safety checks.

## Classification and scope

The three scout reports in this directory are candidate inventories. The accepted
removal record is `removals.md`; candidate recommendations are not blanket approval.

- Remove exact decorative colours, radii, spacing and SVG path snapshots.
- Remove empty-helper, controller-inheritance and Ruby `Data` default tests where
  application callers already exercise the useful behaviour.
- Trim decorative assertions from mixed examples without losing routes, content,
  state, permissions, translations or accessibility assertions.
- Keep stock-warning thresholds, timing, units, touch targets and actual browser
  overflow regressions. A CSS assertion is not automatically low value.
- Defer shared-wrapper consolidation, per-icon currentColor checks and ambiguous
  responsive rules. Do not change production code to make deletion convenient.

The UI sweep is committed at `3f52caff`. This tranche changes only tests and audit
records. No fixtures, application code or browser regression tests are owned here.

## Team and acceptance

Nightingale (Luna) is the sole test writer. Root accepts exact removals and owns
verification and delivery. Sol independently reviews the entire deletion diff for
lost behaviour and inappropriate weakening. Findings return to the same writer.

Run the applicable Ruby lint and full Rails suite after accepted deletions, and
record their actual results. Do not infer runtime improvements from test counts.

## Progress

- Classification completed by three read-only scouts and root.
- Bounded removal work completed by Nightingale: 71 examples removed on balance,
  12 obsolete files deleted and 39 other spec files edited.
- Independent Sol review accepted; root accepted after the full suite, lint,
  documentation build and whitespace checks passed. See `verification.md`.
