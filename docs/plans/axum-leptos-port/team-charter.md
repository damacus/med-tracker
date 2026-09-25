# Port team charter

Effective 25 September 2026. This charter governs the current port run.

## Authority and ownership

The user explicitly requested parallel, separate test and implementation
writers. This overrides the single-writer rule in team-development and
team-planning for this run. Test and product writers have disjoint paths.
Each seat retains its own review fixes. Review and test execution are separate
read-only lanes and do not consume a writer seat.

| Seat | Model and effort | Ownership |
| --- | --- | --- |
| Orchestrator | Current root model | Scope, interpretation, shared interfaces, planning records, acceptance, Git integration and publication |
| Test writer | GPT-6 Sol, advertised default medium | Contract tests, test fixture, contract task wiring, own report |
| Product writer | GPT-6 Sol, advertised default medium | Rust API implementation, API manifest and lockfile, own report |
| Runner/scout | GPT-6 Luna, advertised default medium | Read-only checks and bounded discovery; own evidence report only |
| Independent reviewer | GPT-6 Sol, advertised default medium | Requirements and security/code-quality review; own review report only |

The user's subsequent Compose correction adds one bounded tooling writer
(Sol, advertised default medium), owning only the paths in
[compose-runner-brief.md](compose-runner-brief.md). This removes the host-port
constraint without overlapping the OAuth test or product owners.

Use the current checkout for this tranche: test and runtime files have clear
ownership and immediate integration avoids repeated cherry-picks. A worktree
requires a concrete need for incompatible state or isolated reproduction.
Only the orchestrator commits, rebases, pushes, or changes ownership. No agent
edits another seat's files. No global skill or memory changes are included.

## Concurrent TDD

Prioritize complete user journeys over repeated inventory expansion. The next
delivery milestone after the active OAuth slice is authenticated medication
viewing, dose recording, and updated stock/history. Existing Rails tests are
the requirement source; the full parity inventory remains the cutover
checklist. Keep each handoff small enough to implement and review without
turning every assertion into a separate planning exercise.

The test writer establishes a failing behavioural test against unchanged
product code. The product writer can inspect and design concurrently, then
implement that behaviour after the recorded red result. Meanwhile the test
writer develops the next case. Product fixes never weaken acceptance tests.
Known Rails defects are recorded and tested against intended behaviour;
observing a defect does not make it a Rust requirement.

Read-only discovery and review can run concurrently without a writer lock.
Acceptance evidence needs a stable input: writers freeze the relevant files
for the run and the runner records HEAD plus a content digest of relevant
tracked and new source files before and after it. Exclude generated artifacts
and reports. A changing input invalidates that result. The existing HTTP
runner now uses port 39998 inside each isolated Compose project, with the
HTTP test sidecar sharing its own API container's network namespace. There
is no host API port to serialize. On this host, concurrent runs use checked
explicit subnets because Docker's default address pools are exhausted.
Formatting checks, unit tests, and review can run independently.

## Acceptance and escalation

Each brief defines paths, behaviour, checks, report location, and exclusions.
Reports distinguish red, green, static checks, review, and unverified claims.
Independent review must give separate requirements and code-quality verdicts,
including the combined tranche. Important findings return to their file owner.
After two unsuccessful fixes, stop editing and ask the orchestrator to
diagnose the bounded failure. Scope, security, or interface conflicts go to
the orchestrator immediately; do not broaden a task to bypass a blocker.

Use repository Task commands, Fish, restricted PostgreSQL 18 application
roles and disposable data. No live migration, deployment, merge, or external
message is authorized. No source comments are added or removed.

Measure accepted behaviours, time to first red and green, review rework, and
resource waits where available. Test counts and idle RSS alone are not parity
or performance proof. At a safe handoff record exact files, state, processes,
evidence, and next action before releasing ownership.

Record preparation, build/fixture, implementation, review-fix and resource-wait
time where available. Consolidate checks at a stable slice boundary; repeat a
check only when a changed input or unresolved risk makes its result obsolete.
