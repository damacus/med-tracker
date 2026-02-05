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

- **Environment**: ALWAYS use the test environment for testing.
- **Primary Method**: Use `task test` to run the full suite or `task test TEST_FILE=spec/models/user_spec.rb` for specific files.
- **Alternative Local**: Use `task local:test:all` for faster local testing, `task local:test` for non-browser tests, `task local:test:browser` for browser tests.
- **Framework** RSpec (`spec/`) with Capybara system specs; fixtures in `spec/fixtures/` must remain realistic and unique.
- **Coverage** Policy, model, and feature behavior require exhaustive examples; use VCR for external HTTP if introduced.
- **FORBIDDEN**: Never run `bundle exec rspec` directly - always use task commands.

## Tooling & Automation

### CRITICAL: Use Task Files ONLY

- **NEVER use `docker compose` directly** - This causes failures and environment inconsistencies
- **ALWAYS use `task` commands** from Taskfile.yml and included Taskfiles for all operations
- **Task files provide**: Environment consistency, proper variable handling, error prevention

### Essential Task Commands

**Development:**

- `task dev:up` - Start development server
- `task dev:stop` - Stop development server  
- `task dev:seed` - Seed development database
- `task dev:console` - Open Rails console
- `task dev:logs` - View development logs
- `task dev:rebuild` - Rebuild development environment (WARNING: drops data)

**Testing:**

- `task test` - Run tests in Docker (primary method)
- `task test:up` - Start test environment
- `task test:seed` - Seed test database
- `task test:stop` - Stop test environment

**Local Testing (faster, CI-like):**

- `task local:test:all` - Run all tests locally
- `task local:test` - Run non-browser tests locally
- `task local:test:browser` - Run browser tests locally
- `task local:clean` - Clean up local database

**Quality & Utilities:**

- `task rubocop` - Run linter
- `task rubocop AUTOCORRECT=true` - Run linter with fixes
- `task status` - Check project readiness

**Other Tools:**

- **Beads (bd)**: ALL task and issue tracking via Beads. Use `bd list`, `bd show <id>`, `bd create`, `bd close <id>`.
- **Workflows**: Review `.windsurf/workflows/` for task-specific playbooks.
- **Linting**: RuboCop config in `.rubocop.yml`.
- **CI**: GitHub Actions (`.github/workflows/`) run tests and Playwright suites.

## Collaboration Notes

- **GitHub Flow** We use GitHub Flow: create feature branches from `main`, open PRs, merge after review. **Never push directly to main** - lefthook enforces this.
- **Branches & commits** Conventional Commits (`feat:`, `fix:`, etc.), small atomic changes, always green tests before merge. GPG-signed commits are strongly recommended and may be required by future branch protection rules, but are not currently enforced by tooling.
- **Environment** Rails server via `task dev:up`; avoid destructive commands without confirmation.
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

## Local Testing

### PRIMARY METHODS (use these)

- `task test` - Run tests in Docker environment (recommended)
- `task local:test:all` - Run all tests locally with PostgreSQL container
- `task local:test` - Run non-browser tests locally (faster)
- `task local:test:browser` - Run browser tests locally with Playwright

### QUALITY GATES

- `task rubocop` - Run RuboCop linter
- `task rubocop AUTOCORRECT=true` - Fix RuboCop issues
- `task status` - Check project readiness with tests and Beads

### FORBIDDEN

- NEVER use `docker compose` directly - always use task commands
- NEVER run `bundle exec rspec` directly - use `task test` or local testing tasks

**Task Commands**: All operations must use `task` commands from Taskfile.yml to ensure environment consistency and prevent configuration errors.
