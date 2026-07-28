# Contributing to MedTracker

## Quick setup

```fish
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
bin/setup-claude   # installs task, gems, npm packages, Python deps, Playwright
npm install -g portless
portless trust
task dev:portless  # start the dev stack at https://med-tracker.localhost
task dev:seed      # seed the database
```

Open <https://med-tracker.localhost>.

> **Note**: `bin/setup-claude` handles several environment quirks (Bundler/CGI
> proxy issue, offline Playwright browsers, blocked `task` installer). If you
> are setting up in a restricted network or CI environment, run this script
> rather than installing dependencies manually.

## Development workflow

```fish
task test                    # run RSpec suite in Docker
task test TEST_FILE=spec/models/user_spec.rb  # run a single file
task rubocop                 # lint Ruby
task rubocop AUTOCORRECT=true  # auto-fix style issues
task dev:portless            # start / restart the dev server
task stop-all                # stop all Docker environments
task lighthouse:run          # accessibility/performance audit (requires the dev stack)
task docs:serve              # serve docs locally
```

## Standards

### Testing

- Framework: **RSpec** (`spec/**/*_spec.rb`)
- System/integration tests: **Capybara** (`spec/features/`, `spec/system/`)
- Test data: **Rails fixtures** in `spec/fixtures/` and **FactoryBot** in `spec/factories/`
- External HTTP: **VCR** cassettes in `spec/vcr_cassettes/`
- Test through public APIs only (controller actions, public model methods)
- Tests document expected business behaviour, not implementation details

### Code style

RuboCop enforces the standard configuration (`.rubocop.yml`). Key rules:

- Guard clauses instead of nested `if/else`
- Small, single-responsibility methods
- No nested blocks deeper than 2 levels
- No inline comments — write self-documenting code with descriptive names
- Prefer `Enumerable` methods over imperative loops

#### Naming

| Thing | Convention |
|---|---|
| Methods / variables | `snake_case` |
| Classes / modules | `PascalCase` |
| Constants | `UPPER_SNAKE_CASE` |
| Files | `snake_case.rb` |
| Test files | `*_spec.rb` |

### Architecture

- Domain logic lives on the server; the front end renders server-sent HTML via
  **Hotwire** (Turbo + Stimulus)
- Views are **Phlex** components under `app/components/` — not ERB
- Complex business logic that doesn't belong in a model or controller goes in a
  **service object** (PORO) under `app/services/`
- Authorization is handled by **Pundit** — check/update policies when touching
  access control

### Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add prescription management feature
fix: resolve dose timing validation error
docs: update CONTRIBUTING with setup steps
refactor: extract dosage calculation to service object
test: add coverage for medication take model
chore: bump Ruby to 4.0.6
```

- Each commit is a complete, logical unit of work — tests green before merge
- Avoid "initial commit" or "wip" messages

## Stack reference

| Layer | Technology |
|---|---|
| Language | Ruby 4.0.6 |
| Framework | Ruby on Rails 8.1.3 |
| Database | PostgreSQL 18 |
| Frontend | Hotwire, Phlex, TailwindCSS, Propshaft |
| Testing | RSpec, Capybara, Playwright |
| Authentication | Rodauth passwords, passkeys, and OIDC |
| Authorization | Pundit |
| Task runner | [Task](https://taskfile.dev) (`Taskfile.yml`) |
| CI | GitHub Actions (`.github/workflows/`) |
