# Task 3 report: Serial dashboard optimisation

## Outcome

No production optimisation was retained. The measured candidate reduced repeated stock-source resolution and allocations, but improved representative request latency p95 by only 2.85%, below the required 20% gate. The candidate production and regression-test changes were reverted.

No async Active Record queries were introduced. The temporary candidate reused an already loaded, policy-scoped, ordered medication collection and preserved dashboard output, authorization, tenant context, and ordering.

## Baseline

Docker verification completed before profiling:

```text
$ task test:preflight
14 examples, 0 failures
```

The first 2-warmup/10-measured run wrote its artifact to container-local `/tmp`, so the same command and iteration counts were rerun with bind-mounted relative paths for Vernier inspection and the before/after comparison:

```text
$ task profile:dashboard WARMUP_ITERATIONS=2 MEASURED_ITERATIONS=10 OUT=tmp/task3-before.vernier.json.gz SUMMARY=tmp/task3-before.md
HTTP status: 200
Request latency p50: 242.53ms
Request latency p95: 348.53ms
SQL queries p50: 57
SQL queries p95: 57
Allocations p50: 104620
Allocations p95: 104648
```

The inspectable Vernier run covered 6.96318067 seconds with 9,738 samples and 7,711 unique samples.

The earlier container-local run also returned HTTP 200 with p50 254.20ms, p95 288.24ms, SQL p95 57, and allocation p95 104659. Its lower p95 reinforces that the retained/no-retain decision must not treat the later candidate's 338.60ms p95 as a material improvement.

## Dominant measured path

The ranked inclusive app-owned frames in `tmp/task3-before.vernier.json.gz` identified serial dashboard timeline rendering as the dominant application path:

```text
11724  Components::Dashboard::IndexView#view_template
4707   Components::Dashboard::IndexView#render_timeline_section
3753   Components::Medications::TakeAction#render_take_dialog
2580   Components::Dashboard::PersonTaskCard#render_task_row
1776   FamilyDashboard::ScheduleQuery#aggregate_rows
1022   DashboardPresenter#routine_tasks_by_person
1022   DashboardPresenter#action_rows
1015   FamilyDashboard::ScheduleQuery#call
```

SQL markers confirmed repeated medication stock-source resolution within that path. Across the 12 dashboard requests covered by warmup and measurement, the same matching-medication eager load ran 84 times for Paracetamol, 36 times for Vitamin D, and 36 times for Ibuprofen. There were 228 `Medication Eager Load` markers in total.

The profile did not contain the experimental dashboard's `entries_for_person` or `timeline_entries` frames, so that unmeasured candidate was not changed.

Serena activated the project successfully, but its Ruby language server later terminated during symbol initialization with `LanguageServerTerminatedException`. Per the repository instructions, subsequent inspection used bounded `rg` and `sed` reads around the measured files.

## Candidate and TDD evidence

The candidate reused `MedicationStockSourceResolver#available_medications` from `FamilyDashboard::ScheduleQuery` when rendering the corresponding `Components::Medications::TakeAction`. The collection was already policy-scoped, ordered by location and medication ID, and materialized as an array. No dialog or rendered controls were removed.

The public dashboard regression spec was written first. After correcting an out-of-stock test-data setup issue, RED failed on the intended duplicate-work contract:

```text
$ task test TEST_FILE=spec/components/dashboard/index_view_performance_spec.rb
expected: 1
     got: 2
2 examples, 1 failure
```

The assertion also verified that both authorized stock-source input IDs remained rendered in location order.

The minimum reuse implementation then passed:

```text
$ task test TEST_FILE=spec/components/dashboard/index_view_performance_spec.rb
2 examples, 0 failures
```

## Identical comparison

The candidate used the same authenticated account, household, selected-person value, warmup count, measured count, and profiler configuration:

```text
$ task profile:dashboard WARMUP_ITERATIONS=2 MEASURED_ITERATIONS=10 OUT=tmp/task3-after.vernier.json.gz SUMMARY=tmp/task3-after.md
HTTP status: 200
Request latency p50: 216.89ms
Request latency p95: 338.60ms
SQL queries p50: 57
SQL queries p95: 57
Allocations p50: 95064
Allocations p95: 95088
```

The candidate Vernier run covered 6.400051836 seconds with 8,253 samples and 6,602 unique samples. `Medication Eager Load` markers fell from 228 to 120.

| Gate | Baseline | Candidate | Change | Result |
| --- | ---: | ---: | ---: | --- |
| Request latency p95 | 348.53ms | 338.60ms | 2.85% faster | Fail: below 20% |
| SQL query p95 | 57 | 57 | No change | Pass |
| Allocation p95 | 104648 | 95088 | 9.14% lower | Pass |

The p95 latency gate controls adoption, so the allocation and repeated-work improvements were insufficient. The production candidate and its temporary regression spec were reverted. Only this no-change evidence is retained.
