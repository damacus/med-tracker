# Dashboard serial optimisation decision

The authenticated full-request Vernier profile identified timeline rendering and take-dialog construction as the dominant application-owned dashboard path. Across 12 profiled requests, medication stock-source eager-load markers repeated 228 times.

A temporary candidate reused the policy-scoped, ordered stock-source collection already loaded while building each schedule row. Its public component regression spec passed and eager-load markers fell from 228 to 120 without changing rendered stock-source order.

Both comparison runs used two warmups and ten measured requests:

| Metric | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Request latency p95 | 348.53ms | 338.60ms | 2.85% faster |
| SQL query p95 | 57 | 57 | No change |
| Allocation p95 | 104648 | 95088 | 9.14% lower |

The project requires at least a 20% representative p95 improvement before adopting a dashboard optimisation. The candidate missed that gate, so its production and regression-test changes were reverted. No async Active Record work was introduced.

The complete commands, profiler frames, RED/GREEN evidence, and comparison are recorded in `.superpowers/sdd/task-3-report.md`.
