# Contributing to MedTracker

## Quick setup

```fish
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
npm install -g portless
portless trust
task dev:portless  # start the dev stack at https://med-tracker.localhost
task dev:seed      # seed the database
```

Open <https://med-tracker.localhost>.

Install Docker, Task, Node.js, npm, and Git before running these commands. The
[technical quick start](docs/quick-start.md) explains the local stack and its
destructive reset command.

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
- Browser tests use **Capybara and Playwright** (`spec/features/`, `spec/system/`)
- Test data uses **Rails fixtures** in `spec/fixtures/` and **FactoryBot** in `spec/factories/`
- External HTTP: **VCR** cassettes in `spec/vcr_cassettes/`
- Test through public APIs only (controller actions, public model methods)
- Tests document expected business behaviour rather than code details

### Code style

RuboCop enforces the standard configuration (`.rubocop.yml`). Key rules:

- Guard clauses instead of nested `if/else`
- Small, single-responsibility methods
- No nested blocks deeper than 2 levels
- Do not change comments unless the task requires it
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

- Domain logic lives on the server. The front end renders server-sent HTML via
  **Hotwire** (Turbo + Stimulus)
- Views are **Phlex** components under `app/components/` rather than ERB
- Complex business logic that doesn't belong in a model or controller goes in a
  **service object** (PORO) under `app/services/`
- Authorization is handled by **Pundit**. Check policies when changing
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

- Each commit is a complete, logical unit of work with green checks before merge
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
