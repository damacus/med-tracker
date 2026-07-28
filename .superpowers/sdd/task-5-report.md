# Task 5: Background-job isolation and bulk enqueue

## Outcome

Daily reminder scheduling now builds Active Job instances with their original
arguments and `scheduled_at` values, then submits the complete set through one
`ActiveJob.perform_all_later` call. After the bulk operation, the scheduler
checks every job's `successfully_enqueued?` and `enqueue_error` state and raises
a `BulkEnqueueError` with failed job details when the adapter reports a partial
failure. Adapter exceptions continue to propagate.

Reminder job construction still runs inside each household's
`TenantContext.with` boundary. Each notification job receives household
identity before person identity and re-establishes its own tenant context when
it executes. No request threads or asynchronous Active Record queries were
added.

## Queue isolation

- `MedicationReminderJob`, `MissedDoseNotificationJob`, and
  `LowStockNotificationJob` use the `notifications` queue.
- `NhsDmdImportJob` and `MedicationReviewEvidenceRefreshJob` use the `imports`
  queue.
- Other jobs continue to use the `default` queue.
- The recurring medication-review evidence refresh now explicitly targets
  `imports`.

Solid Queue has one exact-name worker for each queue. With the default
`SOLID_QUEUE_DATABASE_POOL=8`, the topology allocates three notification
threads, one import thread, and two default threads. The six aggregate worker
threads leave two connections for Solid Queue polling and heartbeat work. The
ERB allocation derives from the configured queue database pool and rejects
pool sizes below five, where three isolated workers plus the two reserved
connections cannot fit.

## Verification

The required Context7 Rails and Solid Queue lookups were attempted first but
were unavailable because the Context7 account had reached its monthly quota.
The implementation was checked instead against the Rails 8.1.3 API
documentation and the locked Solid Queue 1.4.0 README. Those primary sources
confirm that `perform_all_later` accepts configured job instances, exposes
per-job enqueue status, and that worker threads should not exceed the queue
database pool minus the two polling/heartbeat connections.

Red failures proved that the scheduler made no bulk call, lacked enqueue
failure visibility, used wildcard/default queues, and did not isolate the
recurring import. The focused Task 5 run then passed 60 examples covering:

- one bulk call with preserved arguments and scheduled timestamps;
- partial enqueue failure details;
- notification and import queue names;
- queue worker totals against the actual database configuration;
- tenant-context identity ordering;
- existing missed-dose and low-stock duplicate suppression;
- notification/import job behavior.

`task rubocop` found no Task 5 offenses. Its repository-wide exit remains
nonzero because of two pre-existing Task 1 offenses in
`spec/config/audition_toolchain_spec.rb` (`RSpec/DescribeClass` and
`RSpec/ExampleLength`), which were left untouched.
