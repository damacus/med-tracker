# Direct dose entry: test writer report

## Red evidence

At baseline `2a8b4c1b`, the first fixture-backed Rust GET test was added to the canonical medication acceptance selection. `task api:contract-subnet-check CONTRACT_TEST_SUBNET=192.168.240.0/28` passed. `task api:acceptance CONTRACT_TEST_SUBNET=192.168.240.0/28` then compiled and ran the selected Rust tests against the unchanged API. The new GET `/api/v1/households/{id}/medication_takes` failed with HTTP 404 instead of 200; the task exited 201. Its disposable Compose project and fixture were cleaned. This is a behavioural red for the new dose route, not a green claim for the expanded cases below.

## Fixture and coverage

The disposable fixture adds separate 1.25 ml direct-dose sources for ordinary, tracked-dosage, timed, scheduled, foreign-household, and concurrent writes. Browser-only desktop and mobile assignments each start with 20.00 ml and use the existing owner and managed-person grant. The fixture exports their numeric medication and assignment IDs and names. No shared developer data is used.

`dose_write_api.rs` now checks:

- Household-scoped history: owner visibility, restricted viewer exclusion, foreign-household denial, stable ordering, pagination limits, and `updated_since` filtering and rejection.
- Ordinary and scheduled direct creates: canonical 1.25 ml dose, exact stock subtraction, listed history, one take version and change event, and a request audit event.
- Tracked dosage: decrement the selected `dosages.current_supply` by 1.25 and synchronize the medication total.
- Idempotency: unchanged UUID replay returns the original take with no second stock or domain audit change; changed payload and cross-household UUID reuse return conflict without exposing the original take. Concurrent identical writes converge on one take and one stock change. A replay after the source stock reaches zero must still succeed without a new mutation.
- Denials for view-only role, hidden or foreign source, wrong source medication, invalid or future time, conflicting explicit unit, timing restriction, foreign household, and missing token. Rejected writes leave stock, take count, and take versions unchanged; POST requests still receive request audit events.

Database reads in the contract inspect exact decimal stock values and audit rows after HTTP requests. They do not seed the dose writes under test.

## Checks and handoff

`task contract:fmt`, `task contract:clippy`, `task rubocop` (1,892 files, zero offences), and `git diff --check` passed after the test and fixture edits. The expanded Rust cases have not yet run against the developing product implementation. The runner owns the next stable combined acceptance and browser result. A Europe/London monthly-cycle boundary was identified in source review and is being fixed by the product writer; this HTTP suite does not add a timezone-specific fixture.

Fixture inputs `scripts/contract_provision.rb` and `rust/contract-tests/src/lib.rs` are frozen for the Rails browser baseline. Test-owned paths are `rust/contract-tests/tests/dose_write_api.rs`, `rust/contract-tests/run_medication_api_tests.fish`, the two fixture inputs, and this report. No Git mutation or publication occurred in this lane.
