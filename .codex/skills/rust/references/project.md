# Project map

Read the relevant manifest, Taskfile and nearby implementation. Versions and
available tasks can change; this map deliberately does not pin them.

| Boundary | Start here |
| --- | --- |
| Axum API, browser authentication and route orchestration | `rust/api/src/lib.rs`, `rust/api/src/web_pages.rs`, `rust/api/src/web_pages/` |
| Leptos views, view models and static assets | `rust/web/src/` |
| Existing component integration | `rust/ui-preview/`, `rust/web/Cargo.toml` |
| HTTP compatibility and fixture isolation | `rust/contract-tests/tests/`, `rust/contract-tests/README.md`, `rust/contract-tests/run.fish` |
| Browser acceptance | `rust/web/tests/*.test.mjs` |
| API definition | `docs/api/openapi.v1.yaml` |
| Intended behaviour and declared exceptions | Relevant `openspec/changes/` or `openspec/specs/`, `rust/parity-matrix.md` |

Root `Taskfile.yml` includes the API tasks as `api:`. Web and UI-preview have
their own Taskfiles. Inspect the selected definition before executing it;
some acceptance tasks provision containers or rebuild assets. Use the repository
execution rules rather than introducing a parallel host workflow.

Treat old READMEs, parity matrices and handoffs as leads. Confirm claimed
coverage in the current routes and tests. A route's existence does not prove
its permissions, write semantics or browser journey.
