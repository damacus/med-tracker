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

That run completed at approximately 10:33 UTC with exit 201. The two lifecycle cases
passed, including the first scalar dose and stale form after dosage options change.
Owner navigation passed. Viewer navigation stopped during the helper's unrelated
dashboard setup: bearer identity returned 200, login returned 302, then the viewer
dashboard returned 503 instead of 200. Inventory/detail assertions were not reached.
Cargo stopped before `household_web`; browser suites and screenshots were not reached.
Owned project containers, images, network and storage were cleaned successfully.

## Navigation setup correction rerun

After coordinator authorisation, the navigation helper was changed to log in without
requiring the unrelated dashboard. Inventory and medication detail still require 200
and retain all mutation-link assertions. The earlier viewer dashboard 503 remains a
separate observed failure; this rerun does not establish that it is fixed.

Fresh run started approximately 10:43 UTC with the same three API targets, four browser
suites and `10.243.18.0/28` subnet. Project: `mtcontract-0d63c2b8e8dd41e2`.
Run directory: `tmp/contract-tests/run.iQz8Bg`.
Log: `/tmp/medtracker-runner-recovery-rerun.log`.

Immutable digest:
`e59c226990f4caebf91e1c9523e30e863f684f6b24b33d6395a91669e8f87180`.
Build history confirms the Rust image completed in 1 minute 50 seconds. Host disk
remained at 178 GiB free when the rerun completed around 10:47 UTC.

| Target | Result |
|---|---|
| `household_lifecycle` | 2 passed, 0 failed |
| `household_navigation` | 2 passed, 0 failed; viewer inventory/detail assertions actually ran |
| `household_web` | 5 passed, 5 failed |
| Four browser suites | Not reached after API failure; no fresh screenshots |

`household_web` failures were:

- `household_people_keep_dependent_capacity_false_after_a_forged_form_value`: API people read-back returned 401, expected 200, line 287.
- `household_ui_capabilities_follow_owner_and_viewer_permissions`: owner capability API returned 401, expected 200, line 210.
- `medication_identity_update_requires_a_current_browser_precondition`: initial API medication read returned 401, expected 200, line 459.
- `medication_read_exposes_editable_identity_and_warnings_without_losing_data`: API medication read returned 401, expected 200, line 435.
- `view_only_member_cannot_open_create_forms_or_edit_a_visible_person`: its own unchanged helper required viewer dashboard 200 and received 503, line 47. The create/edit denial assertions were not reached.

These are observed HTTP results, not evidence that each intended product assertion
failed. No cause was established by this runner for the API 401 responses. The
dashboard 503 remains separately observed. The overall task exited 201. Its exit
hook cleaned the owned project's containers, images, volumes, network and storage.

## Browser-session read-back correction rerun

Coordinator identified the previous owner API 401 responses as stale fixture bearer
tokens after person creation changed the membership permission version. Owner reads
now use the authenticated browser session, and the household helper reads people HTML
for CSRF instead of requiring unrelated dashboard access. Viewer/delegate/foreign
bearer checks remain separate. Browser workflow read-backs use the page's session.

The same three API targets and four browser suites started again around 10:50 UTC.
Project: `mtcontract-32ef588f2b934b02`; directory: `tmp/contract-tests/run.Q8aa3g`.
Log: `/tmp/medtracker-runner-recovery-final.log`. Disk was 178 GiB free at startup.

Snapshot digest: `d1e538b2a52c152d5c763018ea576e1512daa71284039867dc6561e159d00def`.
Coordinator cancelled this run before assertions because the people index has no CSRF
meta token. This is invalid test setup, not product RED. Interrupt ended the runner
during fixture provisioning. Its exit cleanup did not run, so explicit ownership-checked
`task contract:cleanup` removed that project's services/network/volumes with exit 0.
Build caches were retained. Marker/snapshot/storage files remain as cancelled-run evidence.

Coordinator corrected the helper to use medication inventory HTML, which has the existing
CSRF meta token. A fresh authorised run follows in `/tmp/medtracker-runner-inventory-csrf.log`.

## Inventory CSRF rerun results

Project: `mtcontract-f856c6d7e6a24483`; directory: `tmp/contract-tests/run.ROe9sZ`.
Immutable digest:
`69e7434e64c4ad972f4a81417facd2ee5f47e26f3b63e42b731bd0786d4c9c91`.
All 14 HTTP tests passed: lifecycle 2, navigation 2, household web 10.

Browser output reports 21 tests: 15 passed and 6 failed. Household private routes and
all person/location/manual medication create/edit workflows passed at desktop and mobile.
Those workflows include page overflow assertions, validation draft retention and escaped
description rendering. The invalid credentials/foreign medication guard also passed.

All six existing medication journey failures timed out after 30 seconds waiting for
`getByRole('link', { name: 'Log', exact: true })`: CSRF/replay form, taper schedule,
Escape focus desktop/mobile, and decimal dose/stock/history desktop/mobile. No dose
submission assertion was reached in those six cases. Cause was not established by this
runner. Overall task exited 201 and owned resource cleanup succeeded.

Untruncated browser evidence was copied from RTK tee output to
`/tmp/medtracker-runner-inventory-csrf-browser-raw.log`.

Fresh screenshots are in `docs/screenshots/journey-medication-rust/`, with mtime
approximately 11:55 London on 2026-10-01. They include
`household-person-new-{desktop,mobile}.png`, `household-person-error-{desktop,mobile}.png`,
`household-location-error-{desktop,mobile}.png`, `household-medication-new-{desktop,mobile}.png`
and `household-medication-error-{desktop,mobile}.png`.
Person, location and medication desktop/mobile images were visually inspected: fields,
retained drafts and error messages are legible, and focused fields have visible outlines.
The medication Save button retains native styling. The mobile rail uses horizontal
`overflow:auto`; its last link is partly outside the initial screenshot viewport, while
the workflow page overflow assertions passed. Scroll interaction itself was not tested.

The existing dose-dialog/stock/history screenshots retain their earlier 07:53 London
mtime and are not fresh evidence from this run. This run does not establish broader
stock workflows, dosage option management or full translation completion. Disk had
164 GiB free during browser execution.

## Dose-source projection repair acceptance

Coordinator froze the dose-source projection repair after owner/authentication unit
tests and Clippy passed. Combined acceptance started at approximately 11:08 UTC with
the same three HTTP targets (now 15 cases) and four browser suites.
Project: `mtcontract-3df3dfe4a2a14742`; directory: `tmp/contract-tests/run.DLn0H8`.
Snapshot digest:
`daca74fbc8ac71d5af36357e7f7b59925c0805dc6b98345489cc9b5c3bb15618`.
Log: `/tmp/medtracker-runner-dose-projection.log`. Disk was 168 GiB free at startup.

After snapshot creation, coordinator applied API crate formatting to correct import
ordering for edition 2021. The coordinator confirmed this changed import order only;
runtime behaviour is identical to the immutable snapshot. The snapshot was retained
and verification was not restarted for that formatting correction.

The run completed around 11:12 UTC with all 15 HTTP cases passing (2 lifecycle,
2 navigation, 11 household web). The new dose-source permission/stock projection case
passed. Browser results were 21 tests: 17 passed, 4 failed. All household routes and
desktop/mobile workflows passed. Escape focus now passed on both viewports, showing
that the Log control and dialog were restored.

Four failures reached history display assertions rather than timing out on Log:

- CSRF/replay form: exact `1.25 ml` history text was not visible (test line 203, helper line 101).
- Taper schedule: exact `0.75 ml` history text was not visible (line 264).
- Decimal dose desktop/mobile: exact `1.25 ml` history text was not visible (line 161).

Dose submission and the preceding stock checks completed before these assertions.
The decimal-dose screenshots show 18.75 ml remaining after a 1.25 ml dose from 20 ml.
History rendering remains unverified/failing; no cause was established by this runner.
Raw browser evidence: `/tmp/medtracker-runner-dose-projection-browser-raw.log`.
Overall task exit was 201; owned containers, images, network, volumes and storage were
cleaned successfully.

All ten household screenshots were refreshed around 12:11 London and visually inspected.
Fresh `journey-dose-dialog-{desktop,mobile}.png` and `journey-stock-{desktop,mobile}.png`
were also inspected: the responsive dialog shows a readable 1.25 ml dose and stock-source
selector; the inventory status shows 18.75 ml remaining. History screenshots still have
07:53 London timestamps and are not evidence for this run. Disk remained above 165 GiB
free during the build. Broader stock, dosage-option editing and translation scope is
not established by these bounded results.

## Existing history markup assertion correction

Coordinator changed only history test selectors to match the existing combined person
and dose text in the scoped `small` element, retaining visibility, exact dose suffix,
row counts, stock, replay, CSRF and taper assertions. No renderer change was included.

The first rerun snapshot was
`adabbcfd9a0e631509a5fc6ad01aa9e12a6ece74d60ab683138d9b51cfb82032`, project
`mtcontract-1971fd2f262a403e` (`tmp/contract-tests/run.vL633A`).
Log: `/tmp/medtracker-runner-history-selector.log`.
It exited 201 before Rust startup or assertions: the repeated dashboard UI build could
not load `lines-and-columns` from its shared `node_modules` during Tailwind execution.
This is setup failure, not product RED. Fixture provisioning completed after 58 seconds.
Ownership cleanup succeeded; no fresh screenshots were generated. Disk had 164 GiB free.

Coordinator identified overlap with the runner self-test's real source-snapshot task,
which repeatedly rebuilds shared UI dependencies. After the serial self-test passed,
the coordinator explicitly released acceptance. The retry's digest matched the same
`adabbcfd9a0e631509a5fc6ad01aa9e12a6ece74d60ab683138d9b51cfb82032` snapshot.
Project: `mtcontract-4b3abf953e0b4dd0`; directory: `tmp/contract-tests/run.srY10l`.
Log: `/tmp/medtracker-runner-history-final.log`; raw browser output:
`/tmp/medtracker-runner-history-final-browser-raw.log`.

The retry completed around 11:32 UTC: all 15 HTTP tests passed. Browser results were
21 tests, 20 passed and 1 failed. Household workflows, CSRF/replay, taper, Escape focus,
desktop decimal-dose journey and both exact history amount suffix assertions passed.
The sole failure was the mobile decimal-dose journey's dashboard page overflow assertion:
`document.documentElement.scrollWidth <= window.innerWidth` was false at line 164.
That occurred after its visible scoped `1.25 ml` history assertion passed and before
the mobile history screenshot. This is an observed mobile layout failure.

Fresh `journey-history-desktop.png` (12:31 London) was visually inspected: the recorded
medication appears once with readable person and `1.25 ml` summary. The mobile history
image remains 07:53 London and cannot establish the current mobile layout. The other
household/dialog/stock images were refreshed around 12:31 London. Overall task exit201
and ownership cleanup succeeded; disk remained 151 GiB free. No new product edits were
made by the runner.

## History text wrapping repair verification

Coordinator added `min-width:0`/`flex:1` to history text, anywhere wrapping to the
medication/person text, and no shrinking to the timestamp. The page width assertion
was retained. Independent source review accepted this narrow repair before runtime.

Snapshot: `1b3e5b63535bdf125cf90f2f241b18b37661d0f63bb30a16ff7af517a5683570`.
Project: `mtcontract-b64b13fa175e423b`; run: `tmp/contract-tests/run.1p2Sij`.
Log: `/tmp/medtracker-runner-history-layout.log`; raw browser output:
`/tmp/medtracker-runner-history-layout-browser-raw.log`.

At approximately 11:40 UTC, all 15 HTTP cases passed and browser results remained
20 passed, 1 failed. The mobile decimal-dose dashboard still violated the unchanged
global page width assertion at line 164, after its history amount assertion passed.
The candidate has not resolved the observed mobile overflow. Mobile history capture
was not reached. The runner automatically cleaned its owned resources; buffered
browser output appeared during cleanup, preventing a separate live bounds capture.
Coordinator was told that diagnostic bounds/screenshot capture before the assertion
is required to identify the offending element without weakening the assertion.
Overall task exit201; cleanup succeeded. Disk remained 146 GiB free.

## Narrow overflow geometry diagnosis

Coordinator added screenshot and element bounds capture before the unchanged assertion.
Only `medication-journey.test.mjs` ran, with household HTTP targets disabled.
Snapshot `8a79baa9f3743b0ea952fdb024a0dec7a880dcdb2b029b3d730e1e52444080be`;
project `mtcontract-42da76533c714adb`; run `tmp/contract-tests/run.ibapDS`.
Log `/tmp/medtracker-runner-mobile-geometry.log`; raw browser evidence
`/tmp/medtracker-runner-mobile-geometry-browser-raw.log`.

Four cases passed and mobile overflow failed. At width390 the page scrollWidth was471.
History/insights sections had width455/right455/min-width:auto; history timestamps
extended to right437. Stock bars/items/refill button had width407/left24/right431.
The fresh `journey-history-mobile.png` (12:47 London) was visually inspected: recorded
person/dose text is readable, but cards/timestamps extend past the viewport's right edge.
This establishes ancestor sizing overflow rather than merely an unreadable dose label.
Cleanup succeeded; no production edits were made by the runner.

## Final ancestor grid repair: GREEN

Coordinator changed the single-column responsive grid track from `1fr` to
`minmax(0,1fr)` and set direct grid children to `min-width:0`, using the captured
geometry. The strict width assertion remained unchanged. Combined acceptance used
snapshot `453d76d198e59dd96a22373c8953a0555feeda615951f87e33a11e2f0d5e96dc`;
project `mtcontract-1c213d1948a54acc`; run `tmp/contract-tests/run.eWuLIq`.

At approximately 11:52 UTC the overall task exited **0**. All **15 HTTP cases** and
all **21 browser cases** passed. This includes household routes/forms, viewer permissions,
decimal-dose submission/stock/history on desktop and mobile, CSRF rejection, replay,
taper, Escape focus and the unchanged mobile page overflow assertion.

Log: `/tmp/medtracker-runner-grid-repair.log`. Untruncated browser output:
`/tmp/medtracker-runner-grid-repair-browser-raw.log`, copied from RTK tee
`1790855539_task_api_9062b8.log`.

All 16 scoped screenshots in `docs/screenshots/journey-medication-rust/` were refreshed
around 12:52 London: ten household images plus desktop/mobile dialog, stock and history.
Fresh mobile history was visually inspected: medication/person text wraps within cards,
timestamps stay visible, and the cards fit the 390 px viewport. Fresh desktop history,
mobile dialog and mobile stock were also inspected. The existing broader stock/options
and full translation follow-up scope is not marked complete by this bounded run.

The exit hook successfully removed only the owned project's services, images, volumes,
network and storage. Disk had 158 GiB free after completion. No production source edits
were made by the runner.
