# GitHub Copilot Instructions for MedTracker

`AGENTS.md` is the authoritative repository guide. Follow its architecture,
testing, security, UI, data-access, command, and verification rules.

## Orientation

- Read `README.md` for the product and local setup.
- Read `docs/design.md` for the current architecture and domain records.
- Use `docs/glossary.md` for medication and care terminology.
- Resolve Rust requirements and dependencies from `Cargo.toml` and `Cargo.lock`;
  do not copy versions from old plans.

## Root Rust application

- Loco is the root application. Work in `src/`, `config/*.yaml`, `assets/`,
  `migration/` and `tests/`. Consult `rust/AGENTS.md` for upstream Loco
  conventions and issue #2575 for deliberate departures.
- The older Axum/Leptos sources under `rust/` are migration inputs, not the
  root application. Preserve existing behaviour and UI during migration.
- Use Loco's `AppContext`, Axum controllers, SeaORM models and Tera browser
  templates. Reuse existing application patterns before adding new abstractions.
- Rust authentication uses the `better-auth` integration under
  `src/models/identity/`. Authorisation uses Cedar policies under
  `src/models/authorization/`, household memberships and person access grants.
  Preserve the access checks, tenant transactions, row-level security and audit
  attribution provided by `src/models/access.rs`.
- Read runtime configuration through `ctx.config.settings`; never read
  `std::env` inside `src/`. Declare environment variables with `get_env` in each
  applicable `config/*.yaml` settings block and pass settings through existing
  controller and model boundaries.
- Add API acceptance tests using Loco `TestServer` over the fully booted
  `App::boot` router. Reuse template-cloned PostgreSQL databases and restricted-role
  fixtures. Preserve ConnectInfo and socket-specific coverage where required;
  never enable global truncation or use administrator credentials at runtime.
- `docs/api/openapi.v1.yaml` is the authoritative API contract for all runtimes
  and native clients. Keep API changes consistent with it.
- Schema adoption, fixtures and production images have explicit migration
  acceptance gates. Do not deploy the foundation as the finished application or
  run verification against a live database.

## Repository contracts

- Use `task` commands for development, tests, linting, security checks, and
  container workflows. Do not bypass them with raw Cargo or Docker Compose
  commands.
- Root `task dev`, `build`, `check`, `lint`, `fmt`, `test`, `routes`, `worker`
  and `browser` commands belong to Loco. Run `task ci` for Loco changes.
- Start the root Loco application with `task dev`.
- Use PostgreSQL 18 in every environment.
- Browser views use Tera templates under `assets/`. Keep forms accessible and
  verify visible UI changes in desktop and mobile browsers.
- Follow Red-Green-Refactor for executable behavior changes.
- For Markdown-only changes, run `task docs:build` and `git diff --check`
  instead of the full application suite.

## Safety

- Treat medication, person, household, authentication, and audit data as
  sensitive.
- Preserve authorization boundaries and attributable medication history.
- Reuse maintained libraries for authentication, security protocols and
  cryptography; do not implement these from scratch.
- Never add real health information, credentials, tokens, or unredacted logs
  to fixtures, documentation, issues, or commits.
