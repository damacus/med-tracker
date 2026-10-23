# Moving MedTracker from Rails to Rust: memory, latency and trade-offs

23 October 2026

Rails is brilliant. Getting an application from an idea to something useful is
exactly where it shines. Moving MedTracker to Rust needs a better reason than
being excited about a different language.

The reason is RAM. We want to run MedTracker on modest hardware without memory
being a constant constraint. Six weeks into the move, it was time to ask whether
the runtime savings were real—and what the work had cost us.

## Give Rails a fair comparison

Comparing an empty Rust process with a busy Rails installation would tell us very
little. We used seeded applications, the same real NHS DM+d catalogue and matching
services: one application container, PostgreSQL and RustFS for each stack. Both
had working sign-in and dashboards. No Mailpit on either side.

We tried Rails with forked jobs, threaded jobs and inline import, plus Loco with
background and inline import. Each configuration ran once. The
[benchmark report](../development/runtime-benchmark-2026-10-23.md) records the
images, data hashes, methods and all five results.

Threaded Rails matters here: production already uses an async worker. Changing
our local forked configuration to threaded jobs is not a newly discovered
production saving. The experiment also combined web and jobs in one application
container; it did not reproduce the separate production web and worker layout.

## Less memory after doing real work

Five minutes after importing the same catalogue, the background configurations
looked like this:

| Measured working set | Threaded Rails | Loco background |
|---|---:|---:|
| Application container, including jobs and scheduler | 908 MiB | 205 MiB |
| Whole stack, including PostgreSQL and RustFS | 1,193 MiB | 517 MiB |

These are Docker working-set readings, not process RSS. The catalogue contents
matched by ordered business-field hashes. Retained memory alone does not show
a leak, and these single local trials do not establish user capacity.

We then gave the combined Loco application a hard **1 GiB limit with no swap**.
It completed the same import, retained matching catalogue contents and recorded
no OOM events or restarts. Working set peaked at about 251 MiB during import.
The kernel high-water mark across startup and observation was about 482 MiB,
including file cache—a different measure from working set.

PostgreSQL and RustFS had no individual memory caps and were outside that
application limit. This proves that the tested application workload completed
within 1 GiB; it does not put the whole stack, or a production deployment,
inside that budget.

Rails also completed a matched threaded import under the same hard 1 GiB
application limit with no swap, no OOM events and no restarts through five
minutes of retention. Its sampled working set peaked at about 989 MiB and
retained about 948 MiB, compared with Loco's 251 MiB and 242 MiB. Rails reached
the kernel memory ceiling and recorded limit-hit events, without an OOM kill.
The result is more memory headroom for Loco, not evidence that Rails cannot do
this job within 1 GiB.

Both trials used uncapped supporting services. Simultaneous whole-stack working
set peaked at about 1,783 MiB for Rails and 549 MiB for Loco during import, with
cache and maintenance activity in reused database containers included. Two Rails
probes timed out at a two-second deadline; none of Loco's six import probes did.
The unequal sample counts and light traffic do not establish a latency
distribution or explain the timeout cause. The
[matched-trial appendix](../development/runtime-benchmark-2026-10-23.md#matched-one-gib-rails-async-follow-up)
keeps the exact counters, timing caveats and supporting-service figures together.

## Less CPU work, rather than a lower peak

A separate follow-up measured CPU work per completed import. Loco background
used 4.74 whole-stack CPU-seconds versus 141.44 for threaded Rails in that trial.
A CPU-second means one logical core busy for one second.

Loco's sampled application peak was slightly higher: about 110% of one core
versus 103%. Its busy period was much shorter. Quiet idle application medians
were lower too, but database and storage activity narrowed the whole-stack gap.
Scheduled activity meant Loco's idle peak was not always lower.

That is useful evidence of less CPU work for this import. It is not an electricity
measurement or a promise about sustained load on another machine.

## Dashboard latency needs more work

The dashboard observations were not uniformly better in Loco. In the original
five-minute post-import windows, sampled dashboard p95 was about **150 ms for
Loco versus 101 ms for threaded Rails**, with 59 requests each. In the bounded
trials it was about **172 ms versus 156 ms**, with 59 and 60 requests respectively.
All those requests succeeded. Lower response time is better, so these are worse
Loco dashboard observations.

The original during-import table is much less useful for comparing tails: Loco
had just two dashboard samples per mode. Their nearest-rank p95 is simply their
maximum. A faster import also leaves less time to collect probes. Health checks
are a different endpoint and must not stand in for dashboard response times.

These were single sequential local runs with light traffic, not a controlled
user-load latency study. They do not establish the cause or repeatability of a
regression. The memory result stands, but we have not established a dashboard
tail-latency improvement. The report now shows the
[phase-by-phase dashboard observations](../development/runtime-benchmark-2026-10-23.md#dashboard-responsiveness-observations)
and sampling limits alongside the resource measurements.

## Some of this is the implementation

The Rails importer saves barcodes individually; Rust batches database writes.
We did not test a Rails importer with equivalent batching. The timing and CPU
differences therefore cannot all be credited to the language.

There was no manual Rust tuning for these trials, but the image was a release
build with compiler optimisation and existing batching. It was also an older
release image with a temporary shell addition for Loco's scheduled tasks, not a
new build of current main. The shell adjustment remains a packaging limitation.

The migration delivered other useful changes: validated dose and treatment
commands, current care-authority checks and explicit portability boundaries.
Those are design choices we could also make in Rails. Rust did not invent them,
and moving typed data through the application is still incremental.

## The harder bits were real too

Rails gives a team a familiar vocabulary. In Rust we have had to agree more
explicitly on request data, validated commands, domain operations, authentication,
membership and care authority. Communicating that architecture has been hard.
That is a shared design problem; a common vocabulary matters as much as types.
The [fresh Rust architecture](../adrs/0016-fresh-rust-without-legacy-compatibility.md)
and [authentication boundaries](../adrs/0015-authentication-membership-and-tenancy.md)
record those decisions.

Explicit conversions and error contracts also take work. The typed-dose change
needed a [follow-up correction](https://github.com/damacus/med-tracker/pull/2874)
to preserve conflict responses for malformed retries. Types help constrain an
operation; they do not replace behaviour tests. Authentication plugins,
transactions, PostgreSQL row-level security and async process cleanup still need
deliberate integration.

Build resources are another cost. A current-source release build coincided with
severe memory pressure in a shared 8 GB Docker VM and was cancelled. Recovered
records suggest it was a major contributor, without proving exclusive ownership
of the pressure or an isolated build-memory requirement. That incident was not
an application import failure. Lower runtime RAM does not automatically mean
cheaper builds, quicker feedback or lower delivery cost.

## What this means for 0.6

The measurements support our reason for moving: lower operating RAM on the
workload we tested. They do not settle the entire cost of the migration or make
Rails a bad choice. We have evidence worth keeping, alongside limitations worth
remembering.

MedTracker 0.6 starts with a fresh database and new credentials. Clinical portable
transfer is separate, compatibility work remains open and there is no supported
rollback into Rails. The [release notes](../releases/0.6.md) explain those
boundaries. This article is a record of the work so far, not a deployment
announcement.
