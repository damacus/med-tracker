# MedTracker Agent Guide

## Mission

- **Overall goal** Manage and track medication schedules so carers and patients can confidently record doses and respect safety rules.
- **Primary outcome** Accurate, auditable history of prescriptions and over-the-counter medicines for every `Person`.
- **Source of truth** Domain logic lives on the Rails server; the front end renders server-sent HTML via Hotwire.

## Orientation

- **Read first** `README.md` for setup, `docs/design.md` for architecture, `USER_MANAGEMENT_PLAN.md` for roadmap.
- **Key directories** `app/` (Rails MVC + Phlex components), `spec/` (RSpec and Capybara tests), `config/` (environment + routes), `db/migrate/` (schema history).
- **Named guides** `PERSON_MEDICINES_IMPLEMENTATION.md` documents the non-prescription flow.

## Domain Model Highlights

- **People & Users** `Person` stores demographic data; `User` handles authentication and roles (administrator, doctor, nurse, carer, parent).
- **Medicines** `Medicine` plus `Prescription` define formal regimens; `PersonMedicine` covers ad-hoc supplements; `MedicationTake` logs every dose.
- **Relationships** `CarerRelationship` links carers to patients, enforcing capacity support.
- **Constraints** Timing rules (`max_daily_doses`, `min_hours_between_doses`) enforced in models before UI.

## Application Design

- **Backend** Ruby on Rails with service-style POROs when business logic grows; guard clauses preferred for early exits.
- **Frontend** Hotwire (Turbo + Stimulus) with Phlex components under `app/components/`; dialogs lean on HTML `<dialog>` and Turbo Streams.
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

- **Branches & commits** Conventional Commits (`feat:`, `fix:`, etc.), small atomic changes, always green tests before merge.
- **Environment** rails server via `bin/dev`; avoid destructive commands without confirmation.
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
