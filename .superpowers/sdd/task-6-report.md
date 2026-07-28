# Task 6 report: CPU/Ractor decision gate

## Decision

No production Ractor code was added. The completed measurements do not identify
a CPU-bound, primitive-data batch that is eligible for a Ractor comparison.
Because the candidate gate failed before implementation, no synthetic
serial/process/Ractor benchmark was created.

The measured dashboard path remains a normal Rails request. External lookup
batches remain bounded threads, and application batch work remains isolated in
background jobs and their worker processes.

## Candidates considered

### Dashboard timeline rendering

Task 2 measured the complete authenticated request at 343.62 ms p50 and
412.50 ms p95, with 57 SQL queries and about 104,500 allocations. Task 3's
representative 10-iteration profile identified Phlex component rendering,
schedule aggregation, presenter work, and repeated Active Record medication
loading as the dominant application-owned frames.

This is the only measured path with apparent application CPU work, so it was
the sole Ractor candidate considered. It was rejected before benchmarking:

- it is an interactive request, not a primitive-data batch;
- the path operates on Rails models, policy-scoped collections, Phlex
  components, request context, and database results;
- the measured serial candidate improved p95 by 2.85%, below its 20% gate; and
- moving only a fragment behind serialization and transfer overhead would not
  address the measured SQL and object-graph work.

The target scan below found no static issue in the dashboard component scope.
That is a necessary readiness signal, not evidence that the live Rails object
graph can cross a Ractor boundary or that Ractor would improve latency.

### External API batches

Task 4 measured eight deterministic 20 ms NLM requests at 0.1762 seconds with
one worker and 0.0435 seconds with four workers, a 75.3% improvement. The work
is external-I/O wait, not CPU work. The retained bounded ordered thread mapper
is therefore the appropriate boundary; Ractor startup and message transfer
would not remove the network wait.

No unprofiled import, export, report, or maintenance path was promoted into a
candidate merely because it can run in batches.

## Audition evidence

Docker preflight passed:

```text
$ task test:preflight
14 examples, 0 failures
Test preflight passed: spec/config/taskfiles_spec.rb
```

The measured dashboard entry point passed the non-mutating target scan:

```text
$ task audition:target TARGET=app/components/dashboard/index_view.rb
* audition 0.2.1 ruby 4.0.6 · script at app/components/dashboard

  summary: no findings
  verdict: ok ractor-ready as far as audition can tell
```

The repository's dynamic task is bounded by Audition's configured 60-second
timeout. It could not assess Ractor readiness because the tools container
failed to boot Rails without a PostgreSQL socket:

```text
$ task audition:dynamic
./config/environment.rb
  x Rails failed to boot: ActiveRecord::ConnectionNotEstablished:
    connection to server on socket "/var/run/postgresql/.s.PGSQL.5432" failed:
    No such file or directory

dynamic probes
  x rails probe failed (details above)

summary: 1 error
verdict: x not ractor-ready
```

No Audition fixes or baseline updates were applied. The dynamic limitation
does not change the decision: the measured path already fails the CPU-batch
and primitive-data prerequisites.

## Gate result

| Gate | Result |
| --- | --- |
| Evidence-backed CPU hotspot | Fail |
| Primitive-data batch boundary | Fail |
| Target static Audition scan | Pass, with no findings |
| Dynamic Audition probe | Inconclusive: Rails could not boot without PostgreSQL |
| At least 20% faster than best non-Ractor path | Not run; prerequisite gates failed |
| Production Ractor adoption | No |

Threads remain limited to bounded external-I/O fan-out. Background jobs retain
tenant reconstruction and database ownership inside each job, while worker
processes provide the appropriate isolation for Rails application batch work.
A future Ractor proposal must begin with a representative CPU profile showing
a pure primitive-data batch, then pass Audition and beat both serial and
process baselines by at least 20% after startup and transfer costs.
