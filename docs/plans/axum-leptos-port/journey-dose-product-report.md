# Direct dose API product report

This report covers the product writer's Rust API paths. The independent canonical acceptance passed 36/36 HTTP cases and 5/5 browser cases after the focused UUID conflict fix; see the [runner report](journey-dose-runner-report.md) for the run manifest and fixture evidence.

## Implemented boundary

- `GET /api/v1/households/:household_id/medication_takes` lists direct takes with current household and person-view scope, `updated_since`, bounded pagination, source and inventory identifiers, and decimal dose serialization.
- `POST` accepts a current `person_medication` or direct `schedule` source. It checks current account, membership and person-record grant; household/source operation; source-effective dose and unit; future time; schedule date; stock source signature and selection; timing limits and overlapping prescriptions. A conflicting submitted dose unit fails without writing. The selected stock row is locked before exact decimal decrement. Tracked dosage-option stock also updates the parent medication total.
- A household row lock and transaction-scoped client UUID lock serialize contenders. A scoped exact retry returns the original take and ETag with a new request audit. A changed retry returns conflict. The database UUID uniqueness constraint remains the final one-winner backstop. The existing take, stock, version events, change event and success request audit commit together. Rejected requests write a separate request audit after the dose transaction rolls back.
- Replay checks the current person-record grant and stored source identity before comparing supplied fields. It uses the persisted optional dose/stock snapshot when those fields are omitted, so later stock exhaustion, pause or default changes do not consume or block the original dose again.

## Data access and risk boundary

Ordinary reads and inserts use SeaORM entities and query builders. Targeted SQL locks the household and selected stock rows and takes a transaction-scoped UUID advisory lock. All dose queries execute under the existing restricted `med_tracker_app` role and transaction-local tenant settings. Database errors are logged generically without query text or bind values.

The API reuses the Rails `medication_takes`, `medications`, `dosages`, `versions`, `api_change_events` and `security_audit_events` tables. No schema change or independent dose ledger was added. The direct API does not implement scheduled-occurrence actions.

## Checks and evidence

- The unchanged baseline `2a8b4c1b` produced genuine HTTP RED: authorized dose history GET returned 404 rather than 200 in the disposable contract runner.
- Focused unit RED/green was recorded for numeric(10,2) upper range and Europe/London month and daylight-saving cycle boundaries. `task api:test` then passed all six API unit tests.
- `task api:check`, `task api:clippy`, `task api:fmt`, and `git diff --check` passed after the implementation and review fixes. The independent reviewer approved the final source boundary.
- The first canonical combined run passed five of six dose HTTP cases. A foreign household's reuse of a globally unique dose UUID returned 500 instead of generic 409. SeaORM reports an empty `INSERT ... ON CONFLICT DO NOTHING RETURNING` result as `RecordNotFound`; the focused insert mapping now returns 409, rolls back, and writes a fresh failed-request audit without looking up the foreign take. The final canonical rerun passed 36/36 HTTP and 5/5 browser cases.

## Known limit

This is the direct medication-take slice. It does not add scheduled-occurrence actions, a first-party browser dose flow, or offline delivery. The browser result above covers the existing secure-entry regression, not a browser dose journey.
