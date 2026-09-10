# Nightingale brief: CI runtime and validation lanes

Implement the approved plan in `plan.md` as the sole writer. Preserve unrelated work and keep every change within the
listed owned paths.

Start each issue with a failing behavioural or contract test. For #1934, capture a comparable baseline before changing
the test workload, then use measured file timings for two isolated non-browser shards and collate their raw coverage.
For #1936, use measured durations; do not invent weights or claim balance from file counts. For issue #2084, cite the
GitHub Actions failure from run `34216574622`, job `102029683849`: Zipflinger failed with
`java.lang.OutOfMemoryError: Java heap space`. Test `--max-workers=1` before any other memory change.

Write `report.md` with:

- the implementation and changed files for each issue;
- each Red and Green command and its exact result;
- before-and-after non-browser and browser-shard timing evidence;
- fresh-image Playwright evidence without `PLAYWRIGHT_BROWSERS_PATH`;
- complete Android task evidence and any remote fresh-runner evidence available;
- self-review, concerns, unresolved risks, and the next safe action.

Do not commit, push, edit issues, or write the independent review. Update the tranche ledger with final evidence, then
return only a compact status after the report is complete.
