# Isolated Compose acceptance runner

Replace host execution of the medication API and HTTP tests with internal
services in each unique contract Compose project, with no published Rust API
port. The test sidecar shares the API service's network namespace and uses
container-local loopback, preserving the harness's remote HTTPS requirement.
This implements the user's networking correction.

The separate tooling writer owns `rust/contract-tests/run.fish`, new runner
Docker/Compose files beneath that directory, runner isolation/failure Fish
tests, `rust/api/Taskfile.yml`, and `compose-runner-report.md` here. Request
orchestrator integration for shared Taskfiles. No auth or fixture edits.

First add a failing runner test, then implement. Preserve project-scoped data,
cleanup and failure status. Reuse build caches without sharing per-run data
or forcing concurrent builds into one mutable target directory. The initial
OAuth red run has completed; run.fish may now change. Coordinate subsequent
runs with the runner. Final proof needs two overlapping disposable projects
using the same internal API port and distinct networks, with evidence that
each reaches its own fixture. Configuration inspection is insufficient.

Use Fish, Task wrappers, current Context7 Compose documentation, and no source
comment changes. No Git mutations. Report red/green, cleanup, and limitations
concisely. Independent review covers the combined tooling diff. No production
deployment image or broader runner redesign belongs in this task.

Host-specific constraint: Docker's default address pools are exhausted by
unrelated projects. Allow an explicit per-run /28 subnet override for verified
unused ranges, without changing daemon configuration or pruning unrelated
networks. Normal runs keep automatic Compose allocation. Final concurrent
proof may use two checked slices of the range released by this run.
