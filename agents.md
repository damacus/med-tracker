# MedTracker Agent Guide

## Mission

- **Overall goal** Manage and track medication schedules so carers and patients can confidently record doses and respect safety rules.
- **Primary outcome** Accurate, auditable history of prescriptions and over-the-counter medicines for every `Person`.
- **Source of truth** Domain logic lives on the Rails server; the front end renders server-sent HTML via Hotwire.

## Orientation

- **Read first** `README.md` for setup, `docs/design.md` for architecture, `USER_MANAGEMENT_PLAN.md` for roadmap.
- **Key directories** `app/` (Rails MVC + Phlex components), `spec/` (RSpec and Capybara tests), `config/` (environment + routes), `db/migrate/` (schema history).
- **Named guides** `PERSON_MEDICINES_IMPLEMENTATION.md` documents the non-prescription flow, `docs/theming.md` for UI and theming rules.

## Domain Model Highlights

- **People & Users** `Person` stores demographic data; `User` handles authentication and roles (administrator, doctor, nurse, carer, parent).
- **Medicines** `Medicine` plus `Prescription` define formal regimens; `PersonMedicine` covers ad-hoc supplements; `MedicationTake` logs every dose.
- **Relationships** `CarerRelationship` links carers to patients, enforcing capacity support.
- **Constraints** Timing rules (`max_daily_doses`, `min_hours_between_doses`) enforced in models before UI.

## Application Design

- **Backend** Ruby on Rails with service-style POROs when business logic grows; guard clauses preferred for early exits.
- **Frontend** Hotwire (Turbo + Stimulus) with Phlex components under `app/components/`; dialogs lean on HTML `<dialog>` and Turbo Streams. Styling follows `docs/theming.md`.
- **Authentication** Cookie sessions, `has_secure_password`, IP and user-agent tracking via `Authentication` concern; future OIDC planned (`docs/design.md`).
- **Admin area** Namespaced controllers (`app/controllers/admin/`) and Phlex views manage users.

## Development Rules (Agents Must Obey)

- **TDD** Follow strict Red-Green-Refactor; no production code without a failing test. See `.windsurf/rules/testing-strategy.md`.
- **Code style** Ruby style guide enforced by RuboCop; prefer self-documenting code. Reference `.windsurf/rules/code-style-and-architecture.md`.
- **Architecture** Use service objects for complex logic, avoid deep controller/model coupling.
- **Typing & safety** Maintain existing type hints; never loosen types to appease linters.
- **Comments** Do not add or remove comments unless explicitly told by the user.

## Testing Expectations

- **Environment**: ALWAYS use the test environment for testing. Use `task test` to run the full suite or targeted `task` commands if available.
- **Framework** RSpec (`spec/`) with Capybara system specs; fixtures in `spec/fixtures/` must remain realistic and unique.
- **Coverage** Policy, model, and feature behavior require exhaustive examples; use VCR for external HTTP if introduced.
- **Workflow** Use `task test` or specific model/feature test tasks defined in the Taskfile. Do not run `bundle exec rspec` directly.

## Tooling & Automation

- **Beads (bd)**: ALL task and issue tracking is done via Beads. Use `bd list` to see open issues, `bd show <id>` for details, `bd create` to file new work, and `bd close <id>` when done. Never use JSON feature files.
- **Taskfile**: ALWAYS use `task` commands from `Taskfile.yml` instead of running bare commands (like `rspec` or `docker compose`). This ensures environment consistency.
- **Workflows** Review `.windsurf/workflows/` for task-specific playbooks (`/test`, `/rubocop`, `/update-dependencies`, etc.).
- **Linting** RuboCop config lives in `.rubocop.yml`; respect enforced cops.
- **CI** GitHub Actions (`.github/workflows/`) run tests and Playwright suites.

## Collaboration Notes

- **GitHub Flow** We use GitHub Flow: create feature branches from `main`, open PRs, merge after review. **Never push directly to main** - lefthook enforces this.
- **Branches & commits** Conventional Commits (`feat:`, `fix:`, etc.), small atomic changes, always green tests before merge. GPG-signed commits are strongly recommended and may be required by future branch protection rules, but are not currently enforced by tooling.
- **Environment** Start the dev server with `task dev:up`; avoid destructive commands without confirmation.
- **Documentation** Markdown must satisfy `markdown-lint-cli2`; keep headings orderly.

## Useful Entry Points

- **Authentication** `app/controllers/sessions_controller.rb`, `spec/features/sessions_spec.rb`.
- **Medication tracking** `app/models/medication_take.rb`, `spec/features/person_medicines_spec.rb`.
- **Admin users** `app/controllers/admin/users_controller.rb`, `spec/components/admin/`.

## Future Plans

- **Roadmap** `USER_MANAGEMENT_PLAN.md` phases detail authorization, admin CRUD, carer tools, and audit logging.
- **Design vision** `docs/design.md` notes eventual OIDC auth, audit trails, mobile support.
- **Open tasks** Check Beads (`beads list`) for assigned work and plan documents for high-level roadmap.

## Tooling

All terminal commands and scripts should be run using fish shell.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```bash
   git pull --rebase
   bd sync
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

## Database Strategy

**PostgreSQL Only**: MedTracker uses PostgreSQL exclusively for all environments (development, test, production) to ensure dev/test/prod parity and leverage advanced features like `citext` for case-insensitive email comparisons, partial indexes, and JSON/JSONB columns.

### PostgreSQL Version

Always use **PostgreSQL 18** (not 17 or earlier) when specifying database versions in Docker Compose files, configuration, or documentation.

## Development Fixtures

The development database is seeded with fixtures from `spec/fixtures/` via `db/seeds.rb`. All test users have password: `password`.

**Critical Loading Order**: accounts → people → users → medicines → dosages → prescriptions → person_medicines → medication_takes (must respect foreign keys).

## Shell Preference

**Always use Fish shell syntax** for all CLI commands and scripts:

- Variables: `set VAR value` not `VAR=value`
- Export: `set -x VAR value` not `export VAR=value`
- Command substitution: `(command)` not `$(command)`
- Conditionals: `if ... end` not `if ... fi`

## Person Types and Capacity

**Person Model Enum Values:**

- `adult: 0` - Self-managing adult
- `minor: 1` - Child requiring parental consent
- `dependent_adult: 2` - Adult requiring carer support

**Key Logic:**

- **Minors**: `person_type: 1`, `has_capacity: false` - always lack capacity due to age
- **Dependent Adults**: `person_type: 2`, `has_capacity: false` - adults requiring carer support
- **User role** (authentication) vs **Person type** (care requirements) are distinct concepts

## Accessibility Requirements

**WCAG 2.2 SC 2.5.8**: Interactive targets must be at least **24x24 CSS pixels**, with 44x44px recommended for touch targets. Use `min-h-[24px]`/`min-w-[24px]` minimum, `min-h-[44px]` for important controls.

## Quick Commands for PR Review

```bash
# Fetch PR details
gh pr view <PR_NUMBER> --comments
gh pr view <PR_NUMBER> --json title,body,comments,reviews,files | bat -p

# Filter Copilot reviews
gh pr view <PR_NUMBER> --json reviews --jq '.reviews[] | select(.author.login == "copilot") | .body' | bat -p
```

**Common Copilot Issues:**

1. Enum comparisons: Use predicate methods (e.g., `person_type_adult?`) instead of string comparisons
2. Association names: Verify correct association names in models
3. Type mismatches: Ensure types match (symbol vs string, etc.)

## Interacting with the Application

The app runs in Docker. Use `task` commands to manage it — never run `docker compose` or `bin/dev` directly.

### Development Server

| Command | Purpose |
|---|---|
| `task dev:up` | Start the development server |
| `task dev:stop` | Stop the development server |
| `task dev:build` | Build Docker images (first-time or after Gemfile/package changes) |
| `task dev:rebuild` | Rebuild and reset the database (destructive) |
| `task dev:port` | Print the host port the dev server is bound to |
| `task dev:open-ui` | Open the running app in the default browser |
| `task dev:logs` | Tail development server logs |
| `task dev:ps` | Show Docker Compose stack status |
| `task dev:db-migrate` | Run pending database migrations |
| `task dev:seed` | Seed the database with fixtures (all test users: password `password`) |
| `task stop-all` | Stop all services (dev, test, prod) for this worktree |

### Taking Screenshots for PRs

Use `playwright-cli` (the `playwright-cli` skill) to capture screenshots:

```bash
# 1. Start the dev server
task dev:up

# 2. Get the port it's running on
task dev:port   # e.g. 3000

# 3. Open a browser and navigate
playwright-cli open http://localhost:<port>

# 4. Log in (all fixture users have password: password)
playwright-cli goto http://localhost:<port>/login

# 5. Take screenshots
playwright-cli screenshot --filename=screenshot.png

# 6. Close when done
playwright-cli close
```

Attach screenshots to PRs via `gh pr comment <PR> --body "![desc](screenshot.png)"` or upload them as part of the PR body.

### Production Server (local validation)

| Command | Purpose |
|---|---|
| `task prod:up` | Start production server locally for validation |
| `task prod:stop` | Stop production server |
| `task prod:build` | Build production Docker images |
| `task prod:rebuild` | Rebuild production server (drops database) |
| `task prod:logs` | View production server logs |
| `task prod:ps` | List production container status |
| `task prod:seed-users` | Send invitations to initial users |

## Local Testing

Use PostgreSQL for testing via `task test` (Docker) or `task local:test:all` (local). Use `task local:test` for non-browser tests and `task local:test:browser` for browser tests with Playwright. Use `task rubocop` for linting.

**Task Commands**: Always use `task` commands from `Taskfile.yml` instead of running bare commands to ensure environment consistency.
