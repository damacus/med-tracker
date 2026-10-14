# MedTracker Agent Guide

> Keep `AGENTS.md` and `agents.md` in sync. `CLAUDE.md` is a separate Claude Code guide.

## Skill source

Maintained skills live in [damacus/skills](https://github.com/damacus/skills)
and are installed globally. Do not create or edit skill definitions in this
repository, including generated OpenSpec copies under `.codex/skills`.
Keep project facts and commands here; edit reusable workflows in the skills
repository, validate them there, then refresh their global installations.

Use `medtracker-rust` for this project's Rust migration work and the global
`openspec-*` skills for specification work. Use `migrate` to establish the
reference and approved differences, then `team-slice-development` for delivery.
`adaptive-model-routing` owns model selection. The old
`team-tranche-development` and `team-development` names are retired.

## Mandatory First Steps

- **Serena MCP** — For coding, review, or architecture tasks, use tool discovery for `serena initial_instructions` if Serena tools are not already visible, then call Serena `initial_instructions` before broad code exploration or implementation. Activate the project if needed. Prefer Serena symbolic navigation for code structure; if Serena is unavailable or lacks the needed tool, say so briefly and continue with `rg`, `sed`, and normal repo tools.
- **Ruby skill** — For Ruby or Rails coding, review, or debugging tasks, load the Ruby skill before implementation and follow the applicable reference files.
- **Context7** — Fetch current documentation with Context7 before answering or implementing library, framework, SDK, API, CLI, or cloud-service usage details.

## TDD

Follow Red-Green-Refactor — no production code without a failing test first.

1. **Red** — write a test that fails
2. **Green** — write the minimum code to make it pass
3. **Refactor** — clean up while keeping tests green

## Non-obvious rules

- **Shell** — Fish syntax only (`set VAR value`, `(cmd)`, `if … end`)
- **Comments** — Never add or remove comments unless explicitly asked
- **Person enum** — `adult:0`, `minor:1`, `dependent_adult:2`; minors/dependent_adults always `has_capacity:false`
- **PostgreSQL 18** — Use version 18, not 17, in all configs and docs
- **Fixture password** — All dev/test fixture users have password `password`
- **JSON inspection** — Use `jq` for JSON search/filtering; do not write Ruby/Python scripts for ad hoc JSON parsing

## Root application and rollback

Loco 1.2 is the root application: `src/`, `config/*.yaml`, `assets/` and
`migration/`. Rails lives under `rails/` and remains the independently runnable
reference and rollback application during migration. Root `task dev`, `build`,
`test`, `check`, `lint`, `fmt`, `routes`, `worker`, `browser` and `ci` belong to
Loco. Schema adoption, fixtures and production images remain explicit gates until
their migration tranches pass. Never deploy the foundation as the finished app.
The existing `rust/` Axum/Leptos sources remain migration inputs, not root routing.
`rust/` vendors upstream
[loco-rs/loco `skills/loco/`](https://github.com/loco-rs/loco/tree/master/skills/loco):
`AGENTS.md` plus its full reading set (`SKILL.md`, `doctrine.md`,
`workflow.md`, `errors.md`, `starter-app.md`, the `api`/`sea-orm` indexes and
`recipes/`), matched to the pinned `loco-rs =1.2.0`. Consult them when working
on the root Loco app — `doctrine.md` before writing Loco code, the recipe for
the task at hand, and the indexes before guessing at API names. When bumping
the `loco-rs` crate, re-fetch the vendored files from the matching upstream tag.
Deliberate departures from upstream conventions are catalogued below and in
[#2575](https://github.com/damacus/med-tracker/issues/2575).

New API request acceptance uses Loco `TestServer` over the fully booted
`App::boot` router, reusing template-cloned PostgreSQL and restricted-role
fixtures. The ConnectInfo-capable transport exercises the better-auth router,
route layers and HTTP boundary where the client socket address matters. Do not
enable global truncation or replace runtime credentials with administrator
connections. Existing socket tests remain valid; retain their transport-specific
coverage. The Loco testing feature also supplies `EmailSender::stub()` and
`deliveries()` for mail-outbox tests.

Runtime configuration is read through `ctx.config.settings`, never through
`std::env` inside `src/`. Declare each variable in the `settings:` block of
`config/*.yaml` via `get_env`; `access::begin` threads it into
`TenantTransaction` and controllers pass `ctx.config.settings.as_ref()` down.
An empty `get_env` default renders as YAML `null`, and
`models::settings::node()` resolves `null` as absent, so an unset variable
never overrides a stored `app_settings` value.

Entities are generated, never hand-written. `src/models/_entities/` comes from
`cargo loco db entities` over the adopted schema, and `src/models/entities.rs`
re-exports them under the domain names the codebase uses. The five
primary-key-less Rodauth tables remain hand-written, excluded via
`package.metadata.db.entity` `ignore-tables` so regeneration is reproducible.
Regenerate rather than edit after schema changes — generation has already
caught hand-written drift (missing `NOT NULL` bounds and whole columns).

Object storage goes through `ctx.storage`. `models::storage` builds named
`avatar` and `dmd` Loco stores from `ctx.config.settings` and registers them in
`App::after_context`; matching OpenDAL operators live in `ctx.shared_store`
for writes that carry object metadata such as `Content-Type`, which
`StoreDriver::upload` cannot express. Do not add SDK storage clients to `src/`
— `aws-sdk-s3` remains a dev-dependency for test bucket provisioning only.
OpenDAL S3 needs `opendal::install_default()` plus the
`http-transport-reqwest` feature; enabling `services-s3` alone installs no HTTP
transport.

### Deliberate departures from stock Loco

These exist because the Rails-compatible schema, identity stack or migration
gates require them. Do not "fix" them back toward stock Loco without a
migration decision:

- Workers, tasks and mail outbox live under `src/models/` beside their domain
  code (e.g. `models::nhs_dmd::ImportWorker`), not `src/workers/`,
  `src/tasks/` or `src/mailers/` — behaviour stays adjacent to the model it
  acts on.
- Mail delivery enqueues `Email` JSON into `pg_loco_queue` as a transactional
  outbox instead of Loco mailer structs — Rails parity: the mail commit shares
  the triggering write's transaction.
- Identity is bespoke: `better-auth` (git dep), `oxide-auth`, `openidconnect`,
  `webauthn-rs`, `totp-rs`, `cedar-policy`. Loco 1.2 ships no OAuth-server,
  OIDC, passkey or TOTP story.
- Browser sessions use `axum_session` + `axum_session_sqlx` (a second sqlx
  pool) for Rails cookie-session parity, not Loco session middleware.
- `migration/src/fresh/` is a raw-SQL schema-adoption baseline — the schema is
  adopted from Rails, not generated by Loco migrations.
- `Hooks::truncate` returns `Err` — global truncation is a deliberate
  migration gate, not a stub to fill in.
- Routes are hand-wired in `app.rs` with custom `route_layers` /
  `http_boundary` middleware composition — the identity middleware ordering is
  not expressible through stock Loco auth wiring.

## Rails rollback stack

- Ruby 4.0.6
- Rails 8.1.3
- PostgreSQL 18
- Bundler 4.0.3
- RSpec, Capybara, VCR, Rails fixtures
- Playwright 1.61
- RubyUI, Phlex, Hotwire, Propshaft
- RuboCop

## Code and Architecture

### Security protocols and primitives

Do not implement security protocols or cryptographic primitives from scratch
when a mature, maintained library or framework capability exists. This includes
OAuth/OIDC, PKCE, WebAuthn/passkeys, JWT/JWK, signature verification, password
hashing, token/session validation, CSRF protection, authentication challenges,
nonces, state, key parsing and algorithm negotiation.

Before introducing custom security-sensitive protocol code:

1. Search project dependencies and the relevant ecosystem for established implementations.
2. Prefer the mature implementation.
3. Document the exact limitation if it cannot satisfy the requirement.
4. Record why custom implementation is necessary.
5. Add standards/interoperability and negative security tests.
6. Keep the custom implementation as small as possible.

Passing tests does not justify bespoke security code. Do not replace a
battle-tested library with custom logic for perceived simplicity, dependency
reduction, token savings or implementation convenience.

Before implementing a non-trivial infrastructure or protocol capability, check
whether an established dependency or framework feature already provides it.
Prefer adopting and configuring proven implementations unless project
requirements materially prevent doing so.

### Application code

- Use RuboCop as the source of truth for Ruby style.
- Prefer clear names, small private methods, guard clauses, and Enumerable methods where they improve readability.
- Keep controllers focused on HTTP concerns; put business logic in models, POROs, or service objects.
- Use Phlex components in `rails/app/components/`.
- Add nil-safety guards in policy/component code when records or associations may be absent.
- Use `respond_to?` guards where a policy record may be a Class, such as `new` actions.

## Views and UI

- All views are Ruby/Phlex files. Do not create `.erb` files.
- Use RubyUI components when an equivalent component exists, especially for headings, text, links, buttons, forms, cards, tables, dialogs, badges, avatars, separators, popovers, tooltips, and calendar/date inputs.
- Keep UI accessible: keyboard access, labels, alt text, visible focus states, WCAG AA text contrast, descriptive link text, associated error messages, table headers, dismissible focus-trapping dialogs, and logical heading hierarchy.
- For UI work, verify through the actual UI with browser automation and capture desktop/mobile screenshots when the change is visible.
- Fetch current RubyUI docs with Context7 when component API details are unclear.

## Data Access and Performance

- Do not execute database queries inside view components or loops.
- Eager-load associations used by views with `includes`, `preload`, or `eager_load` in the controller/query boundary.
- Pass preloaded associations or query results into components instead of re-querying in nested methods.
- Use `size` on loaded collections instead of `count`.
- Use database filtering/sorting (`where`, `order`) for unloaded ActiveRecord relations.
- Use in-memory filtering (`select`, `reject`) only on already-loaded/eager-loaded collections.
- Prefer `exists?` over `find_by` when only checking existence.
- Wrap related writes in `ActiveRecord::Base.transaction` and use bang persistence methods inside transactions.

## Native mobile workspace

- Rails remains under `rails/`; its `app/` and Docker configuration are Rails-owned.
  The shared root `docs/api/openapi.v1.yaml` is authoritative for every runtime; do not open the root as an
  Android Gradle or Xcode project.
- Route Android work to `mobile/android` and its nested Gradle build after the
  Android application lands. Add root Android Task commands and CI only with
  that application.
- Route iOS work to `mobile/ios` and its nested Xcode build after the verified
  full-history import lands. Add root iOS Task commands and CI only with that
  application.
- The root OpenAPI document is authoritative. Native clients use their own
  explicit pinned copies and generation checks; ordinary native builds do not
  generate from live Rails source.

## Commands

Use `task` for everything. Run `task ci` for Loco changes; the following commands
apply to the Rails rollback application. Rails root entry commands use `rails:`;
`task --dir rails ...` is supported for standalone rollback work. Never run `docker compose`, `bin/dev`, or `bundle exec rspec` directly.

> **Note**: For most `task rails:dev:*` commands, an equivalent `task rails:test:*` command exists (e.g., `task rails:test:up`, `task rails:test:port`).

| What                               | Command                               |
|------------------------------------|---------------------------------------|
| Run tests                          | `task rails:test`                     |
| Test Docker preflight              | `task rails:test:preflight`           |
| Lint                               | `task rails:rubocop`                  |
| Start dev server                   | `task rails:dev:up`                   |
| Build dev images                   | `task rails:dev:build`                |
| View dev logs                      | `task rails:dev:logs`                 |
| Stop dev server                    | `task rails:dev:stop`                 |
| Get dev port                       | `task rails:dev:port`                 |
| Open in browser                    | `task rails:dev:open-ui`              |
| Seed database                      | `task rails:dev:seed`                 |
| Migrate                            | `task rails:dev:db-migrate`           |
| Rebuild (destructive)              | `task rails:dev:rebuild`              |
| Run Brakeman                       | `task rails:brakeman`                 |
| Run RuboCop autocorrect            | `task rails:rubocop AUTOCORRECT=true` |
| Stop everything                    | `task rails:stop-all`                 |
| Run local Playwright browser tests | `task rails:playwright`               |
| List all tasks                     | `task -l`                             |

## Docker Development

- Development uses a bind mount, so Ruby, config, lib, spec, and database file changes sync into the container automatically.
- Rebuild after changing `rails/Gemfile`, `rails/Gemfile.lock`, `rails/package.json`, `rails/yarn.lock`, or Docker configuration.
- Use `task rails:dev:db-migrate` after migrations.
- Use `task rails:dev:rebuild` only for a destructive fresh start.
- Do not use Docker Compose watch; the bind mount and Rails reloader already provide live updates.

## Testing

Documentation-only changes (`*.md` with no executable code or application
configuration changes) do not require `task rails:test:preflight`, the full RSpec
suite, system or Playwright tests, Lighthouse, or application end-to-end tests.
Verify Markdown correctness with `task docs:build` and `git diff --check`, plus
any narrower documentation-specific check relevant to the changed files.

Run `task rails:test:preflight` only before implementation work that changes Rails code. If it reports that Docker is unavailable or the test image is missing, fix that specific prerequisite. Use GitHub CI as the Rails verification authority only when local Docker remains unavailable.

Do not run `task rails:test:preflight` or `task rails:test` for changes confined to CI,
Android, iOS, documentation, plans, or non-Rails tooling. Run the checks relevant
to those areas instead. Mixed changes require Rails tests only when they also
change Rails code.

- Write RSpec tests in `_spec.rb` files using Rails/RSpec conventions.
- Test public APIs and observable behavior, not implementation details.
- Use Rails fixtures in `rails/spec/fixtures/`; keep fixture relationships realistic and avoid duplicate unique attributes.
- Use VCR cassettes in `rails/spec/vcr_cassettes/` for external API mocking.
- Policy changes need explicit coverage for relevant roles: admin, clinician, self, carer, parent, and unauthorized users.
- New model validations need positive and negative test cases.
- Admin CRUD flows need success, validation error, duplicate handling, and immediate-usability coverage.
- Capybara tests should use exact labels/text from the views. Prefer `click_link` for links and `click_button` for buttons.
- Specs that use browser features need `:browser` or `type: :system`.

## Screenshots for PRs

```fish
task rails:dev:up
task rails:dev:port          # → e.g. 3000
# then use the playwright-cli skill to navigate and screenshot
```

Save PR screenshots under `docs/screenshots/` with page and viewport in the filename, for example `dashboard-desktop.png` and `dashboard-mobile.png`.

## Quality gates (run before every push)

Run `task rails:test` only when Rails code changes. Run `task rails:rubocop` when Ruby code
changes. Changes to CI or other non-Rails executable code or configuration do
not, by themselves, require the Rails suite. Run each changed area's relevant
checks; for documentation-only changes, use the verification rule above.

```fish
task rails:rubocop          # lint — must pass with no offenses
task rails:test             # full test suite in Docker — must be green
```

When developing a Rails change, run a single file with:

```fish
task rails:test TEST_FILE=spec/path/to/file_spec.rb
```

Never push if an applicable required check fails.

## Review and Security

- Code review findings should prioritize correctness, missing coverage, authorization gaps, N+1 queries, nil safety, race conditions, unsafe SQL, mass assignment, and existing pattern violations.
- PR review comments must be checked against the current code before changing anything; do not blindly apply Copilot or bot suggestions.
- Address each actionable PR review comment directly after pushing the fix or explain why no code change was needed.
- Security review should use `task rails:brakeman` and manual review of authentication, authorization, strong parameters, model validations, raw SQL, secret handling, security headers, dependency risk, audit trails, and medication/health-data access controls.
- Document false positives or accepted risks before ignoring security findings.

## PR and Commit Text

- Use Conventional Commits with scopes when useful, for example `fix(medicines): handle missing reorder threshold`.
- Keep changes atomic and focused.
- For PR titles and squash messages, use the same Conventional Commit style.
- PR summaries should be human-readable first: explain what problem the change solves, why the change exists, and what future bugs or confusion it prevents. Write for someone passing by the PR who does not already know the implementation details.
- For refactors or infrastructure work, describe the before/after contract in plain language, for example: "this used to be handled differently in several places; now one shared rule handles it consistently."
- Do not include routine test sections in PR descriptions; CI is the source of truth. Mention verification only when it is manual, unusual, blocked, or not covered by CI.
- Include screenshots for visible UI changes.

## Session close (mandatory)

```fish
git pull --rebase
git push
```

Work is not done until `git push` succeeds.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run applicable quality gates** - Use the changed-area rules above; run `task rails:test` only when Rails code changes
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```fish
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
