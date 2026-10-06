# P1 runner and foundation corrections

P1 provides an owned PostgreSQL test runner. It does not adopt the Rails schema,
restore the baseline, migrate medication features or accept the persistence slice.
The implementation passed independent review and full local Loco checks.
The separately bounded foundation corrections still require hosted Linux checks.

## Runner behaviour and inputs

`scripts/migration/run-slice.mjs` uses the existing foundation database lifecycle
and process cleanup helpers. `task slice:test TARGET=persistence` supplies its
child with the explicit endpoint of a unique disposable PostgreSQL 18 fixture.
Ambient database and Compose settings cannot select that fixture's resources.
Success and child failure both return through owned cleanup.

Root `task test` runs the four runner regressions, then unrestricted
`cargo test --locked` through `slice:test-all`. Unit tests, discovered integration
tests and doc tests remain enabled. The existing HTTP smoke retains its separate
owned database fixture. `tests/persistence.rs` connects through Loco/SeaORM and
checks the PostgreSQL version, database and user before its optional forced
failure. This is a connection and lifecycle check, not schema equivalence.

`migration/baseline.sql` preserves the retained PR #2419 artifact byte for byte,
including COPY metadata. Its SHA-256 is
`f5043a97e60cea912a2f36dec79ee99c2d19e5a950916d1508f76d3e0b87966b`.
[Schema provenance](schema-provenance.md) records the exact source revision and
migration comparison. P1 does not execute this artifact.

## Verification

The public `slice:test-runner` RED in verifier session 34235 was behavioural:
the Cargo child received the unusable ambient database URL, and forced child
failure made no owned setup or cleanup calls. It was not a missing import or
compiler failure. These tests used synthetic executables and touched no real
database or Docker resources.

All four synthetic cases subsequently passed: ambient URL isolation, forced
failure cleanup preserving unrelated resources, unrestricted Cargo arguments
for unit/integration/doc tests, and exact baseline hash. The verifier also
completed real temporary PostgreSQL success and forced-child-failure runs,
including owned-resource cleanup. Formatting and Cargo lint passed after the
formatting-only fixture correction. The final root wiring includes these
regression tests in ordinary CI.

The reviewed P1 manifest is
`/private/tmp/medtracker-p1-freeze-20261005.sha256`, SHA-256
`8a634fd3fcbab24be75ca2bf5bc610a418f78761b092e63628b0a9561001a68b`.

## Foundation corrections

Published CI run 37363675318 exposed two setup failures. The workflow policy job
ran Fish-based runner tests without installing Fish. A missing-executable probe
confirmed `status: null` with `ENOENT`. The policy job now installs Fish before
`ci:check`.

The household contract job failed writing `/app/tmp/local_secret.txt`. The
workflow already supplied the host UID to the image. A clean checkout has no
tracked `rails/tmp` parent; the nested storage mount can create that parent
before the Rails process. The actual Fish runner's synthetic clean-directory
regression failed at the write boundary with exit 43 instead of expected 42.
The runner now creates `rails/tmp` as the invoking host user before container
startup. No Rails application behaviour changed. The workflow suite passed
91 tests after these two targeted corrections.

Fresh disposable Rails copies exercised the actual Compose mounts, UID-matched
image, local-secret write and minimal boot. Both old and corrected cases passed
on Docker Desktop. This supplies local startup evidence, but does not reproduce
the Linux daemon ownership mechanism. Hosted Linux remains the regression gate.

## Review and remaining gates

Devin SWE-2 Max reviewed the immutable supplied-source packet, with separate P1
and foundation-correction verdicts. The
single input is
`/private/tmp/medtracker-p1-foundation-review-20261005/review-input.md`, SHA-256
`e801b5e76f3caa36e0b2e76d15824531a12c5015f05d71035de5984b1bf4723a`.
It includes copied helpers, runner, tests, Compose and Task interfaces and the
tracked diff. No live credentials or environment files are included.

Review, local startup, inventory/preservation checks and full Loco CI have passed.
The corrected published hosted gates remain pending.
The earlier published setup head is
`9e499003f9b321a355a133c6a976f42278ab836f`; local corrective checks do not make its
hosted run green. P2/P3 adoption remains gated on accepted setup. No slice is
accepted by this report.

## Supplemental review corrections

The first review output stopped before completing its verdict. The same
conversation's completion supplied requirements-met verdicts and approved
source quality with findings. Three bounded findings received corrections:
`uuid-runtime` is installed with Fish; fixture and Cargo environments remove
the five Docker endpoint/configuration variables; and every fixture lifecycle
Task uses the quoted absolute root Compose file with `--env-file /dev/null`.

The new public runner regressions first failed because Cargo inherited Docker
settings and `up` lacked the pinned Compose path (four of six passed). The
workflow test first failed its missing `uuid-runtime` assertion. These were
synthetic executable tests; they contacted no remote Docker service. All
focused and static checks subsequently passed before the final path-quoting
correction. The same argv regression passed on that final input.

The final P1 manifest SHA-256 is
`a4de2f7a4e36c4998dbdf1d94908cc8eb4a3474a07bc490263cbe373acfa9dd4`.
The narrow immutable re-review input is
`/private/tmp/medtracker-p1-review-fixes-20261005/review-input.md`, SHA-256
`b57e39f55f4321344305c0d430e7b77a6204f800f99c334efda2c510e39559fc`.
It supplies source bodies and diffs for these corrections. Existing bounded
signal handling remains unchanged; no force-exit before cleanup was added.

The verifier's actual Docker Desktop permission probes booted successfully
with both the old absent-parent setup and the corrected host-created parent.
Exact probe resources were cleaned. The corrected boot is positive evidence,
but the Linux ownership cause was not reproduced locally. Corrected hosted
Linux CI remains authoritative for that failure. No speculative ownership
repair was added. Re-review and full local/published gates still precede
acceptance.

The narrow Devin SWE-2 Max re-review (session 55931) closed G1, F1 and F4
without new defects. The final quoted-path runner check passed six of six
cases, satisfying the review's final-input caveat. `ci:test` passed 91 of 91;
`ci:check`, formatting and Cargo lint passed; the existing owned-database
helper tests passed four of four. Inventory and preservation checks also
passed. The preservation ledger's changed entries include the upstream Rails
Gemfile.lock MCP update and other content changes, so they are not exclusively
path or tooling edits.

Full `task ci` passed on the final frozen source in session 32802, including
unrestricted Cargo tests, the real PostgreSQL persistence test, all six runner
regressions, foundation contracts and the actual HTTP health/Tera smoke.
No owned fixture containers, volumes or networks remained afterward.
Log: `/Users/damacus/Library/Application Support/rtk/tee/1791230995_task_ci.log`.
Documentation and whitespace checks passed. The corrected published hosted gates
remain pending. Docker Desktop evidence does not establish the Linux ownership
failure's cause. Root retains acceptance and publication ownership; this report
accepts no schema adoption.

## Baseline artifact whitespace metadata

The coordinator's full staged whitespace check failed with exit 2 when the
previously untracked baseline became staged. The exact upstream artifact has
trailing spaces in original comment lines 50, 64 and 78, plus a blank EOF.
Earlier unstaged whitespace checks did not include this new file.

The existing per-path `.gitattributes` convention now applies
`migration/baseline.sql -whitespace` only to this preserved artifact. No SQL
bytes or comments changed. Its independently read SHA-256 remains
`f5043a97e60cea912a2f36dec79ee99c2d19e5a950916d1508f76d3e0b87966b`.
The coordinator owns staging and the final full staged whitespace/hash checks.
