# Independent household CI promotion review

The household CI matrix now has fourteen isolated journeys, including separate
jobs for dose, schedule and standalone-login compatibility. All use fresh
fixtures. The login job has no preceding HTTP suite; dose and schedule browser
checks only read existing records. Static requirements and quality/security
review pass, and all 79 local CI policy tests pass. Actual
matrix runtime and remote CI remain pending. Detailed receipts are linked below.

This review is read-only and covers the coordinator-owned workflow, policy
and policy tests. It does not accept the unfinished G product or claim that
the new CI matrix has executed successfully.

## Selected journeys and fixture boundaries

The matrix in `.github/workflows/ci.yml:243` selects these existing contracts:

| Matrix row | HTTP selection | Browser selection and clock |
| --- | --- | --- |
| People, locations, medications | Original three targets plus completion medication, then completion dashboard | Seven accepted household/medication files; real clock |
| Dosage options | household_dosage_options | Dosage option CRUD/immediate-administration file; real clock |
| Stock | Three stock targets through HOUSEHOLD_STOCK_ACCEPTANCE | Stock and mixed-unit selector files; real clock |
| Treatments | household_treatment_acceptance, all 37 cases | Five-locale editors and real-time administration files |
| Calendar | HTTP disabled | Calendar file only; March 29 presentation clock |
| Taper times | HTTP disabled | Shared-time file only; March 29 presentation clock |
| Dosage permissions | household_dosage_permissions only | Default two fresh-login medication files; real clock |
| Stock permissions | household_stock_permissions only | Default two fresh-login medication files; real clock |
| Treatment permissions | household_treatment_permissions only | Default two fresh-login medication files; real clock |
| Inventory completion | household_final_acceptance: assignments 2, inventory 4, People collections 1, source errors 2, adjustment audit 6; 15 active cases | Inventory-completion browser file; final expanded composition runtime pending |
| Minor person access | household_minor_readiness only | Minor-readiness browser file; isolated preparation/restoration and local acceptance passed; final matrix runtime pending |
| Dose compatibility | doses only | household-routes browser file; real clock and fresh fixture; composed runtime pending |
| Schedule compatibility | schedules only | household-routes browser file; real clock and fresh fixture; repaired schedule runtime pending |
| Standalone login | HTTP disabled | login.smoke browser file only; real clock and fresh fixture; isolated local acceptance passed, remote matrix pending |

Browser paths are space-separated, matching the reviewed public Task wrapper.
Explicit target precedence, stock selection and completion dashboard ordering
remain in `rust/api/Taskfile.yml:870`. Each matrix job invokes the public
`api:browser-rust` entry point once. `run.fish:172` creates a unique directory
and random project, so each permission-changing row owns a separate database,
session secrets and fixture. Those mutations cannot invalidate another row's
bearer credentials. The two default medication browser files authenticate with
fresh primary cookies; they do not reuse the mutated member's bearer token.

Only the calendar and shared-time rows specify CONTRACT_DASHBOARD_NOW. The
treatment administration row receives an empty clock, preserving actual
recording time. The existing separate rust_dashboard_browser job still runs
its fixed-clock dashboard suites. Calendar HTTP acceptance is explicitly
disabled, as is the standalone-login row, while all write rows retain it.
Checkout-owner UID forwarding matches
the existing runner setup, and fail-fast:false allows all row outcomes to be
observed without making a failed row successful.

The final composition and browser files are installed. The minor browser uses
the reviewed fixture-only preparation and restoration wrapper, with its own
isolated row; it does not use public profile/person-edit setup. Its local HTTP
and five-language browser checks passed, with provenance qualifications in
the final followups report. Publication still requires the final expanded
composition, final gates and published CI. Static selection approval does not
replace those runtime checks.

## CI Success and coverage

`scripts/ci/policy.json:5` adds rust_household_browser to the mandatory Rust-port
job set. `ci.yml:731` keeps always() on CI Success and adds the matrix job to
needs. `gate.mjs` requires every selected job result to equal success; failure,
cancellation, skipped or missing results produce exit status one. No
continue-on-error is present in the new matrix. The new policy test loop checks
all four failing result classes for the household job. Existing selection
classification and other mandatory suites are preserved.

The initial matrix test at `scripts/ci/tests/policy.test.mjs:81` checked global
substring presence rather than the critical per-row associations. The
coordinator strengthened it during review: calendar/taper rows require the
clock and disabled HTTP, other rows reject both clock and acceptance overrides,
treatments require the accepted target and real administration file, and each
permission row requires its own target without browser/core/completion/stock
overrides. The row parser matches the actual workflow indentation and fails
missing required rows through the assertions. Existing default selection and
mandatory-gate tests remain intact. Static review of that initial improvement
passed before execution evidence was available.

Quality/security verdict: static PASS. No new credential exposure, external
messaging, clinical policy
change or permission bypass is introduced by this wiring. I have not run the
policy tests, CI or any runtime command; actual RED/GREEN receipts and the
published CI results must be recorded by the sole verifier/coordinator.

## Final composition and minor row update

The coordinator changed inventory-completion to the reviewed seven-case
`household_final_acceptance` composition and added minor-person-access as a
separate row. It selects `household_minor_readiness` and only
`tests/household-minor-readiness.test.mjs`. Each public runner invocation still
owns its own database and fixture; no minor mutation is placed in the composed
inventory/assignment/People group.

The small policy-test update asserts both exact target names and their browser
paths in the corresponding parsed rows. It rejects completion/stock overrides
on the minor row. The existing non-clock-row assertion still rejects clock and
HTTP-acceptance overrides, and mandatory CI Success policy is unchanged.
Static requirements and quality/security verdicts: PASS for this update.

The coordinator reports an actual assertion RED with 75 passes and one intended
failure, followed by `task ci:check` with all 76 passing after the workflow
change. These are coordinator-reported execution results; I independently read
the current workflow and assertion source, not those raw logs. Actual final
browser/group acceptance and minor fixture preparation remain pending. This
report is frozen after this bounded update, subject to the later fixture wiring
handoff if it changes the reviewed executable contract.

## Installed minor preparation wiring

Static requirements and quality/security verdicts: PASS. The installed
`contract-browser-rust`, `contract-browser-node`, `contract-minor-viewer-sql`
and `contract-minor-wrapper-test` Task blocks exactly match the separately
reviewed private draft. The installed wrapper is unchanged; its nine-case test
only adapts the fake Task path to `fixtures/browser_minor_fake_task.fish`.
Captured source, owned fixture checks, atomic SQL preparation/restoration and
returned-error propagation therefore retain the reviewed contract.

CI now runs `task api:contract-minor-wrapper-test` only for the
minor-person-access row, after prerequisites and before the real browser task.
The policy test requires that exact condition and command. Matrix targets,
clock isolation, ordinary browser defaults and mandatory CI Success behaviour
are unchanged. The coordinator reports the added assertion failed first
(75 passes, one failure), then all 76 policy checks passed after wiring.
Those runtime counts remain coordinator-reported; this review independently
compared installed source and the policy assertion.

Installed wrapper execution, runner lint/selector checks and the fresh prepared
minor browser/restoration packet remain pending with the verifier. No forced
termination restoration guarantee or overall readiness acceptance is added.
This report is frozen again after the bounded installed-wiring review.

The first real minor packet exposed a missed static setup issue: the SQL Task's
Fish precondition used an unescaped end-anchor dollar sign inside double quotes.
Its HTTP case passed, but SQL and browser assertions did not run. Root reproduced
the parse error and escaped only that dollar sign. Static review accepts the
correction: the project pattern, allowed phases and ownership flow are unchanged.
Root reports the private corrected precondition accepts valid prepare and rejects
production project/unsupported phase. The prior static PASS is qualified by
this missed parse issue; fresh real minor-browser acceptance remains pending.
This report is frozen after the narrow correction review.

## Isolated corrected-dose CI row

Static requirements and quality/security PASS for the new dose-compatibility
row. It selects only the doses Cargo target and the explicit read-only
household-routes browser file. It has no fixed clock, completion/stock selector,
filter or acceptance override. The existing matrix environment forwards the
exact target and browser filename into the same self-provisioning wrapper, so
this row receives its own real-clock fixture rather than sharing another dose,
calendar or permission journey. The route suite contains one parent test and
five nested checks. It uses a fresh owner cookie and performs clinical reads only.

The new policy test requires the exact target and one browser filename and
rejects all listed conflicting overrides. The household matrix remains required
by the existing CI Success policy, so this row cannot fail without failing that
gate. Root reports the missing-row test failed first (76 passes, one failure)
and task ci:check later passed all 77 tests after the minimal row was installed.
Those results are coordinator-reported here, not independently executed by this
reviewer. Actual remote CI and all broader G publication gates remain pending;
this static ruling does not accept the private Rust extraction.

The missing-row policy regression first failed with 76 passing checks and one
intended failure, as recorded in the
[RED receipt](/private/tmp/household-g-20261002/G-CI-DOSES-POLICY-RED-001/task.raw.log).
That RED remains coordinator-reported in this review. I independently read the
[GREEN receipt](/private/tmp/household-g-20261002/G-CI-DOSES-POLICY-GREEN-001/task.raw.log):
tests 77, pass 77, fail 0. Its SHA-256 exactly matches
200b1e5b3b871fbdf9b214aae83c35baaaa11c720c45b6c499b676a24543cab1.
This confirms local policy verification. Actual composed runtime for the
twelfth row and remote CI remain pending; passing standalone dose baselines do
not establish either result. This report is frozen after this documentation-only
update.

## Isolated schedule compatibility CI row

Static requirements and quality/security verdicts: PASS. The thirteenth row
selects only the schedules Cargo target and tests/household-routes.test.mjs.
It has no clock, filter, completion, stock or acceptance override. The existing
matrix environment and public runner give it a separate real-clock fixture;
the browser file performs clinical reads rather than another clinical write
journey. Mandatory CI Success handling is unchanged, so a selected failure
still fails that gate.

The policy test now checks dose and schedule rows separately with their exact
target and browser filename. It rejects conflicting selector and clock fields.
I independently read the actual
[RED log](/private/tmp/household-g-20261002/G-CI-SCHEDULE-RED-001/ci-check.raw.log):
78 tests, 77 passes and one missing-schedule-row failure, process exit 201.
After the workflow row was installed, the
[GREEN log](/private/tmp/household-g-20261002/G-CI-SCHEDULE-GREEN-001/ci-check.raw.log)
records 78 passes, zero failures and process exit zero. Their SHA-256 values are
285b551f912d431b72ba369d9675adfe3bc3d809b9f2b6a72d29dd4bab646eab
and 506745c141fe73cf72923c372fcb2d7e5c933673e03d7ebdad65cebdc5faf472.

This accepts local policy verification and the static CI wiring. It does not
claim the repaired schedules target or the remote matrix has passed. Those
behavioural and publication checks remain separate. This report is frozen
after the bounded thirteen-row update.

## Isolated standalone-login CI row

Static requirements and quality/security verdicts: PASS. The fourteenth row
selects only tests/login.smoke.test.mjs with HTTP acceptance false. It has no
HTTP target, fixed clock, filter, completion or stock selector. The same public
runner provisions a fresh fixture, so the required login proof is separated
from other authentication or clinical suites. Mandatory CI Success handling
remains unchanged.

The general matrix assertion now permits an acceptance override for this named
row as well as the two calendar rows. Its separate strict assertion requires
exactly false and the one login browser file, rejecting target/clock/filter/
completion/stock fields. The exception therefore does not silently disable
another write row's HTTP tests or permit a fixed clock here.

I independently read all three actual receipts. The
[initial RED](/private/tmp/household-g-20261002/G-CI-LOGIN-RED-001/ci-check.raw.log)
has 79 tests, 78 passes and one missing-row failure, exit 201; SHA-256
a5d6528289c5269e86da90d1323dc05b5ffa05ae410b3b3b6721e1fa8e4a69b4.
The first attempted
[GREEN](/private/tmp/household-g-20261002/G-CI-LOGIN-GREEN-001/ci-check.raw.log)
also failed 78/79 because the general rule still rejected the new row's
acceptance field, exit 201; SHA-256
d564d6962c48cf05bc476e536f300796282b9ca568c7d7477ababac54a1875e3.
The corrected
[GREEN](/private/tmp/household-g-20261002/G-CI-LOGIN-GREEN-002/ci-check.raw.log)
passes all 79 with exit zero; SHA-256
288042bd586df3682a45e50c821a8daa0bb21817a1069b8a8bd717174be2d862.

This accepts local policy verification and static wiring. The corrected login
browser run and remote fourteen-row CI execution remain pending. The original
login browser failures are preserved in the separate follow-up review; this
wiring does not turn them into successful runtime evidence. This report is
frozen after the bounded fourteen-row update.

The corrected standalone login file subsequently passed all seven browser
cases in its own fresh real-clock fixture, outer exit zero. I independently read
[the receipt](/private/tmp/household-g-20261002/G-OAUTH-BEFORE-LOGIN-SMOKE-FIXED-001/login.raw.log),
SHA-256 d36ffd3b0c3bf8603fe3fd9249d1fa695aabf6d782066eae1a7e320ab7cde9e4,
and verified the installed test matches the reviewed candidate. Source and
fixture identity are runner-emitted only, as qualified in the follow-up review.
This accepts the isolated local selection; it does not claim remote matrix
success. The CI report is frozen after this receipt update.
