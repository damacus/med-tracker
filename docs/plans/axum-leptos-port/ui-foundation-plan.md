# Leptos SSR login foundation

Spec: [plan.md](plan.md), bounded Leptos foundation exception.

## Global constraints

- Use an isolated worktree and branch based on the coordinator's recorded
  integration commit. This slice owns only `rust/web/**`. Do not change root
  workspace files or lockfiles, the shared router, API contracts or fixtures,
  `rust/parity-matrix.md`, database files, or Rails product code.
- Follow the repository's Fish and `task` command rules. Add a local Taskfile
  under `rust/web/` for build, test, and smoke commands if needed. Do not add or
  remove source comments.
- Fetch current Leptos and Axum documentation with Context7 before using their
  APIs. Use one standalone Rust crate and the least integration needed to
  serve a server-rendered page. Keep dependencies inside this crate.
- Use Red-Green-Refactor. Verify the browser test against disposable Rails and
  record an expected failure against an absent Rust target before writing the
  server. Then make the same smoke pass against the Rust server.
- This slice proves a foundation, not authentication, workflow, PWA, mobile
  layout, or complete visual parity. Do not claim those outcomes.
- Check the author, committer, and signing identity before committing as
  `Dan Webb <dan.webb@damacus.io>`. Do not change signing globally. The
  coordinator owns review, integration, and any push.

## Task 1: Public login page SSR smoke

Port the existing login-page example in `spec/system/user_sessions_spec.rb`
(`describe 'login page'`, `it 'displays the login form with all fields'`) into
a target-independent Playwright browser smoke under `rust/web/tests/`. In a
fresh browser context at `/login`, preserve its assertions inside `main`:
email field, password field, `Sign In to Dashboard` button, and `Forgot?`
link. Also preserve `Welcome back` and absent `Continue with` OIDC button from
`spec/features/security/oidc_security_spec.rb` for the no-credentials
configuration. Use accessible locators for the fields and controls. Run at
desktop 1400×900 and mobile 390×844. Run the portable contract against an
isolated Rails test server and record the exact baseline result. Point it at
the absent Rust target and record the connection failure as expected red.

Create the smallest standalone `rust/web/` Rust crate that serves `/login`
through Axum with HTML rendered by a Leptos component on the server. It may
include a route-local health probe for startup readiness. Keep the form
semantically labelled and server-rendered without JavaScript. Do not implement
credential submission, sessions, API calls, persistence, or PWA assets in this
slice. Avoid presenting an inert submit as working authentication: use a
clearly disabled submit until the shared auth integration task is approved,
and have the smoke check its visible label rather than submission behaviour.

Run the same ported browser smoke against the Rust server at both viewports, plus
crate formatting, Clippy, and Rust unit/integration checks through its local
Taskfile. Record server RSS for warm idle as a measurement only if practical;
the under-200 MB target is a later combined-process acceptance gate. Commit
the reviewed-scope files and write a report with red/green commands, outputs,
changed paths, and unresolved integration work. The coordinator will assign
root workspace/router/API integration after independent review.
