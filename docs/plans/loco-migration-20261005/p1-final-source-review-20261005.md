# Supplemental review: G1, F1, F4 closure

Static review of the corrective packet only. Each item is checked against the copied diffs; this verifies closure of prior findings, not a fresh architecture pass.

## G1 — `uuid-runtime` in `policy_check`

**Requirements verdict: met.** The gap was `run.fish`'s unconditional `uuidgen` call with no `uuid-runtime` install in `policy_check`. The step at `.github/workflows/ci.yml` (~line 77) now runs `sudo apt-get install --yes fish uuid-runtime`, and the existing guard at `scripts/ci/tests/contract_paths.test.mjs:12` asserts `apt-get install[^\n]*\buuid-runtime\b` inside the `policy_check` job slice, with the ordering assert at line 13 retained. The regex requires `uuid-runtime` on an `apt-get install` line, which the single-line install satisfies. Stated RED (extended test failed before the edit) is consistent with the assertion.

**Code-quality verdict: closed.**

- Residual prerequisites (`openssl`, `shasum`, `realpath`, `mktemp`) are standard `ubuntu-latest` content; the reviewed tests only reach `uuidgen`, `mktemp`, `realpath` on the `doses` path. Acceptable.
- Nit (non-blocking): the step name "Install Fish for contract runner tests" understates its scope now. Renaming to e.g. "Install contract runner prerequisites" would match the household/dashboard step naming convention, but is cosmetic.

## F1 — `DOCKER_*` scrubbing at both boundaries

**Requirements verdict: met — exceeds the letter of the original requirement.** `foundation-database.mjs:38` deletes `DOCKER_HOST`, `DOCKER_CONTEXT`, `DOCKER_CONFIG`, `DOCKER_TLS_VERIFY`, `DOCKER_CERT_PATH` from the fixture-task environment, and `run-slice.mjs:9` deletes the same five from the Cargo child. This is exactly the list recommended in F1 and covers the redirect class.

**Code-quality verdict: closed.** The test coverage is correct and fails closed:

- The docker shim records per-call `docker_environment_removed` (`run-slice.test.mjs` shim, ~line 28), so `up`, `port`, and `down` are each verified — not just the first call.
- The Cargo branch records `child_docker_environment_removed` (~line 36), and the new test (lines ~69-77) asserts it alongside every docker call. Stated RED (Cargo inherited the vars) matches the assertion structure.
- `withProcessFixture` injects hostile values for all five variables (spawnSync env, ~line 47), so a partial scrub fails.

Non-blocking completeness note: `DOCKER_TLS` (the non-verify enable flag) and `DOCKER_API_VERSION` are unscrubbed, but neither can select an endpoint without `DOCKER_HOST`/`DOCKER_CONTEXT`, so residual exposure is nil. Optional symmetry only.

## F4 — pinned compose file and `--env-file /dev/null`

**Requirements verdict: met in the copied source.** All three public foundation tasks now pin `docker compose -f "{{.ROOT_DIR}}/compose.yaml" --env-file /dev/null -p {{.FOUNDATION_PROJECT}}` (`Taskfile.yml` ~lines 77, 81, 85). Correctness of the mechanism:

- `-f` is a top-level compose flag valid before `up`/`port`/`down`; the absolute path also pins the project directory to the root, so no other compose file can merge.
- `--env-file /dev/null` disables `.env` auto-discovery per current Docker docs as the packet states; `/dev/null` parses as an empty env file. `compose.yaml` contains no `${}` interpolation, so nothing is lost.
- Quoting `"-f \"{{.ROOT_DIR}}/compose.yaml\""` survives a `ROOT_DIR` containing spaces; the test asserts the post-shell-parse argv (`call.args[indexOf('-f') + 1] === join(root, 'compose.yaml')`), so quoting correctness is verified by the assertion itself rather than by inspection.
- Both `indexOf` probes fail closed: a missing `-f` or `--env-file` yields `args[0]` (`'compose'`), failing the equality. Stated RED (up argv before the fix) is consistent.

**Code-quality verdict: closed, subject to the packet's own caveat.** The quoted-path variant is what the copied `Taskfile.yml` shows; the packet says the argv regression recheck is in flight. Since the assertion inspects parsed argv, a successful re-run fully discharges this — flag the recheck as a verification gate, not a code concern. `/dev/null` is POSIX-only, consistent with the already POSIX-bound cleanup (`detached` + `kill(-pid)`).

## Regression scan of the deltas

- Adding `args` to docker call records does not disturb the earlier `operation` sequence assertions; `-p` remains present exactly once for the project-regex shim check.
- Scrubbing `DOCKER_CONFIG` forces `~/.docker` for the fixture pulls; `postgres:18-alpine` is public, so pulls need no ambient credential config — the isolation trade-off is deliberate and correct here.
- `test` task composition, baseline hash test, `slice:test` public interface, and bounded-cleanup semantics are untouched.
- The two manifest SHAs (`a4de2f7a…` P1, `63a09b27…` copied-source) cannot be verified from the packet; recorded as stated values only.

## Verdict summary

G1, F1, F4: **closed** — each fix matches the recommended minimal correction, has RED→GREEN evidence consistent with the assertion structure, and introduces no new defects.

Still open by design (not regressed): **F2/F3** bounded-interruption semantics intentionally unchanged; **G2/G3** — no chown or residue handling added, and the packet correctly reports that Docker Desktop success is positive but does not reproduce the Linux root-owned-parent mechanism, so hosted Linux CI remains the gate. F4's final recheck must pass before merge.

Evidence limits unchanged: baseline bytes/hash, lockfile and manifest identities, `foundation-http.test.mjs` internals, `ci:check` wiring, and hosted CI results are unverifiable from copied sources; no schema adoption or production boot is claimed.
