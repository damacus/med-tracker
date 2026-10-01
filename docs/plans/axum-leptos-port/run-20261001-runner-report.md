# Runner report

Baseline revision: `026c27e2ff8888b5c2455fd2eccf19dc419d763a`.
RED immutable source digest: `d78ff0355198b24dce59b8792116af8d7df001ae8688b6b40ad54b4267979cbd`.
Run directory: `tmp/contract-tests/run.dhyfyC`; project: `mtcontract-e620b33638fc4f11`.
Coordinator was notified immediately after `source.sha256` appeared.

| Check | Status | Evidence |
|---|---|---|
| Network diagnostics, sandbox | Task exit 0 but Docker denied; invalid evidence | `/tmp/medtracker-runner-network.log` was replaced by successful escalated diagnostics |
| Network diagnostics, escalated | Exit 0; Docker networks and host routes listed | `/tmp/medtracker-runner-network.log` |
| Subnet `10.243.18.0/24` | Exit 201; validator requires /28 | superseded candidate |
| Subnet `10.243.18.0/28` | Exit 0 | `/tmp/medtracker-runner-subnet.log` |
| `rtk task api:fmt` | Exit 201; current Rust formatting differences | `/tmp/medtracker-runner-fmt.log` |
| Household route browser RED | Exit 201 after 870 seconds; observable assertions failed | `/tmp/medtracker-runner-route-red.log`; untruncated browser output `/tmp/medtracker-runner-route-red-browser-raw.log` |

The runner added no production source. Formatting is read-only; no autocorrect was run.
No parallel API build was started while the browser runner builds its dashboard dependency.

The full runner provisions a synthetic database and fixture, prints its source digest,
then starts Rust and runs selected browser cases. Its exit hook cleans up the owned
Compose project. A task exit without route assertions is infrastructure evidence only.

Persistent setup requirements sent to the coordinator: retain ownership marker and
storage directory; set storage, subnet and runner Compose overlays; create disposable
session/APNs values; prepare the database; start the test fixture server; provision fixture;
start Rust; check `/up`; then run selected browser/API tests. Never reuse unrelated projects.

## Observed RED

The isolated Rust service became healthy before browser assertions ran. `/people`,
`/locations` and `/medications/new` returned 404 where the owner journey required 200.
The authenticated owner UI capability endpoint returned 404. Medication edit reads lacked
`friendly_name`. These are product failures, not setup failures. The updated browser file
was used despite the earlier Rust source snapshot.

The full runner exit hook completed owned-project cleanup and removed its storage directory.
No screenshots were produced by these request-level assertions. The outer rtk output was
truncated; the nested untruncated log was copied into `/tmp` for durable session evidence.

## Integrated acceptance

Started 2026-10-01 08:59:27 UTC. Product/test freeze confirmed by coordinator after
selected household contract compilation and browser syntax passed. Snapshot digest:
`ee1f625dd4357de7dfef110076e6d47f92ec0f4797b42407dc0443fbc9831f74`.
Disposable project: `mtcontract-dfaa00f2263c488a`. Subnet revalidation passed.

```fish
set -gx HOUSEHOLD_ACCEPTANCE true
set -gx CONTRACT_TEST_SUBNET 10.243.18.0/28
rtk proxy task api:browser-rust BROWSER_TEST_FILES="tests/household-routes.test.mjs tests/household-workflows.test.mjs tests/medication-journey.test.mjs tests/medication-journey-form.test.mjs"
```

Log: `/tmp/medtracker-runner-green.log`. The household API target runs before browser
cases within this project. Acceptance remains running until actual assertions finish.

At 09:10 UTC the Rust image build had not produced a result. Docker's read-only
build-history query also blocked. Coordinator found low host disk space and freed
worktree incremental build caches. No acceptance restart or second bootstrap was made.
The duplicated runner diagnostic was interrupted (exit 130) once coordinator confirmed
their own diagnostic was active. This did not interrupt the acceptance build.

## Recovery after disk incident

At 10:11–10:14 UTC, read-only diagnostics found approximately 220 GiB free.
Docker responded again. Its inventory reported two images, eight active containers,
254 volumes (56.25 GB) and no build cache. Historical completed build records remain;
those records do not establish that their images or cache survived recovery.

The old run directory `tmp/contract-tests/run.BlqqHW` retains `owner`,
`source.sha256` and `fixture.json`. Its source snapshot and storage directory are absent.
The ownership marker still identifies `mtcontract-dfaa00f2263c488a` and the recorded
digest remains `ee1f625dd4357de7dfef110076e6d47f92ec0f4797b42407dc0443fbc9831f74`.
No old acceptance build/Compose process appeared in the process diagnostics.

Network diagnostics still list the old project's network on `10.243.18.0/28`.
That subnet must be released through ownership-checked cleanup or replaced with
another validated isolated subnet before a new project can use it. No unrelated
containers, volumes or networks were removed by this runner.

The old run therefore has no reusable immutable source snapshot or finished Rust
image. A fresh run needs a new source digest and application build after the source
freeze. Docker health and restored disk space are infrastructure evidence; household
API and browser acceptance remain unverified until assertions finish.

At 10:15 UTC the recovery inventory identified seven running Mailpit containers.
The old owned project had only `mtcontract-dfaa00f2263c488a-mail-test-1`; no Rails,
PostgreSQL or Rust container survived for that project. The other six containers
belong to unrelated projects and were left alone.

After coordinator authorisation, `task contract:cleanup` verified the old run's
ownership marker and exited 0. It removed that project's Mailpit container, test
bundle/node_modules volumes and network. The original `10.243.18.0/28` subnet is
therefore available for the fresh run. The marker, digest and fixture files remain
as recovery evidence.

## Fresh acceptance after source freeze

Coordinator confirmed source freeze at approximately 10:19 UTC. The runner started
with all three API targets (`household_web`, `household_navigation`,
`household_lifecycle`) and the four browser files shown above, using the released
`10.243.18.0/28` subnet.

Run directory: `tmp/contract-tests/run.cwgz0M`.
Project: `mtcontract-f081e25c9b444c20`.
Immutable source digest:
`76655c4da501d5d2bcb934942323cf31c84078de477684c560528f2aaaf21bb4`.
Live log: `/tmp/medtracker-runner-recovery.log`.

At 10:22 UTC, dashboard UI compilation finished, isolated PostgreSQL was healthy,
and the disposable database was created. The missing Rails test image was rebuilding.
The local-image pull failure fell back to the declared build. No API or browser
assertions had run at this point.

At the original 10:27 UTC assessment deadline, Rails was healthy (176 seconds to
server readiness) and the disposable fixture was ready (220 seconds). The Rust
contract-runner image remained building. None of the three household API targets or
four browser suites had reached assertions; no fresh screenshots were available.
The deadline result is **acceptance incomplete**, not a passing product result.
The current authorised verification continues to its completion boundary.
