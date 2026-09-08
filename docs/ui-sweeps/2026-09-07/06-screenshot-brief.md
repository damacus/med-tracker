# Final screenshot evidence

Nightingale owns this final bounded capture step before the coordinator runs the full suite.
Use synthetic local test fixtures and the isolated task runner. No Canary evidence.

Capture one matching medication-card layout pair at 390px in light mode, with identical fixture
data and current wording. Label it original layout versus fixed layout: the original layout uses
only the four card/action layout files from baseline `45db2112`, while the fixed layout uses their
current versions. Safely back up and restore the writer-owned files around a temporary capture-only
system spec, or use an isolated baseline checkout. Keep time and data deterministic. Do not run
other test commands concurrently with the temporary layout swap. Always restore the current files,
including if capture fails. Do not retain screenshot-only or bypass examples in the permanent suite.

Save the pair as `docs/screenshots/ui-sweep/card-layout-before-390-light.png` and
`docs/screenshots/ui-sweep/card-layout-after-390-light.png`. The existing initial/intermediate
failure screenshots are historical artifacts and must not be presented as a matching pair.

Also save representative fixed screenshots for profile tabs (320px, enlarged text), location member
controls (390px), household-owner administration (1280px), a single auth alert (390px), and the
long invitation email from step 05 (390px). Reuse
the focused fixtures and existing Playwright screenshot API. Do not submit real recovery emails.

Record paths, fixture/state/viewport details and any limitations in `06-screenshot-report.md`.
Verify the four card/action files are restored to their pre-capture content and `git diff --check`
passes, then return. Root will inspect the images and run the combined gates on the final diff.
