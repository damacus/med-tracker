# Axum and Leptos port ledger

Plan: `docs/plans/axum-leptos-port/plan.md`

Planning baseline: `781c3a297ffe24ae5b70f893d55b568eab1216fc`.

## Decisions

- Observable behaviour parity replaces one-for-one RSpec translation.
- Existing accounts and medical data survive cutover; reauthentication is
  acceptable.
- Combined application-process target is under 200 MB; PostgreSQL and
  supporting services are excluded.
- API tests precede UI tests; product implementation follows both test
  baselines.

## Progress

- Tranche 0: pending.
- Tranche 1: pending.
- Tranche 2: pending.
- Tranche 3: pending.
- Tranche 4: pending.
- Tranche 5: pending.

Record each accepted task as `Task N: complete (commits BASE..HEAD, review
clean)` and each unresolved decision as `Ruling: decision — evidence — cost if
wrong`. Do not mark a task complete from a test run without its independent
review and recorded result.
