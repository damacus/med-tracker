# MedTracker Test Setup Guide

## Overview

MedTracker uses a **Dockerized PostgreSQL-based test environment** with RSpec, Capybara, and Playwright for browser automation. Tests run in containers to ensure dev/test/prod parity.

---

## Core Stack

| Component       | Tool                          | Version   |
|-----------------|-------------------------------|-----------|
| Language        | Ruby                          | 4.0.2     |
| Framework       | Rails                         | 8.0+      |
| Test Framework  | RSpec                         | 8.0+      |
| Browser Testing | Capybara + Playwright         | chromium  |
| Database        | PostgreSQL                    | 18-alpine |
| Test Data       | Rails Fixtures + FactoryBot   | -         |
| Authorization   | Pundit (with pundit-matchers) | 2.0+      |
| Validation      | Shoulda Matchers              | 6.0+      |

---

## Key Files

### Gemfile (test dependencies)

```ruby
group :development, :test do
  gem 'database_cleaner-active_record'
  gem 'capybara'
  gem 'capybara-playwright-driver'
  gem 'factory_bot_rails'
  gem 'pundit-matchers', '>= 2.0'
  gem 'rails-controller-testing'
  gem 'rspec-github', require: false
  gem 'rspec-rails', '>= 8.0'
  gem 'shoulda-matchers', '>= 6.0'
end
```

### Docker Setup

Tests use the `test` stage from the unified multi-stage `Dockerfile` and the `test` profile in `compose.yaml`.

```yaml
# compose.yaml (test profile)
web-test:
  build:
    target: test       # Uses 'test' stage from Dockerfile
  environment:
    RAILS_ENV: test
    DATABASE_URL: postgresql://medtracker:medtracker_password@db-test:5432/medtracker
```

The `test` stage extends the `assets` stage (which has all build tools and gems) and adds Playwright browser dependencies.

---

## RSpec Configuration

### spec/rails_helper.rb

```ruby
require 'rspec/rails'

Rails.root.glob('spec/support/**/*.rb').sort_by(&:to_s).each { |f| require f }

RSpec.configure do |config|
  config.fixture_paths = [Rails.root.join('spec/fixtures')]
  config.use_transactional_fixtures = true

  # System tests use Playwright
  config.before(:each, type: :system) do
    driven_by :playwright, using: :chromium, screen_size: [1400, 1400]
  end

  config.infer_spec_type_from_file_location!
  config.include Pundit::RSpec::Matchers
  config.include FactoryBot::Syntax::Methods
end

Shoulda::Matchers.configure do |config|
  config.integrate do |with|
    with.test_framework :rspec
    with.library :rails
  end
end
```

### spec/support/capybara.rb

```ruby
require 'capybara/rspec'

Capybara.register_driver :playwright do |app|
  Capybara::Playwright::Driver.new(
    app,
    browser: :chromium,
    browser_options: {
      args: [
        '--disable-blink-features=AutomationControlled',
        '--disable-features=PasswordManager',
        '--disable-save-password-bubble'
      ]
    }
  )
end
```

### spec/support/authentication_helpers.rb

```ruby
module AuthenticationHelpers
  def rodauth_login(email, password = 'password')
    visit '/login'
    fill_in 'Email address', with: email
    fill_in 'Password', with: password
    click_button 'Login'
    expect(page).to have_current_path('/dashboard')
  end

  def login_as(user)
    rodauth_login(user.email_address)
  end
end

RSpec.configure do |config|
  config.include AuthenticationHelpers, type: :system
  config.include AuthenticationHelpers, type: :feature
end
```

---

## Task Runner (Taskfile)

```yaml
# Taskfile.yml
tasks:
  test:
    desc: Run tests in Docker
    cmds:
      - task: internal:run
        vars:
          ENVIRONMENT: test
          SERVICE: web
          COMMAND: 'bundle exec rspec {{ .TEST_FILE | default "spec" }}'
```

### Commands

```bash
task test                              # Run all tests
task test TEST_FILE=spec/models/       # Run specific path
task test:rebuild                      # Rebuild containers + DB
task test:logs                         # View container logs
```

---

## CI Pipeline (GitHub Actions)

Two parallel test jobs:

### 1. Non-System Tests (fast)

```yaml
- name: Run non-system tests
  run: bundle exec rspec --exclude-pattern "spec/{system,features,views}/**/*_spec.rb"
```

### 2. System Tests (Playwright)

```yaml
- name: Install Playwright
  run: npx playwright install --with-deps chromium

- name: Precompile assets
  run: bundle exec rails assets:precompile

- name: Run system tests
  run: bundle exec rspec spec/system spec/features spec/views
```

---

## Test Data Strategy

### Fixtures (spec/fixtures/)

- `accounts.yml`, `users.yml`, `people.yml`
- `medicines.yml`, `prescriptions.yml`, `dosages.yml`
- `person_medicines.yml`, `medication_takes.yml`
- `carer_relationships.yml`, `sessions.yml`

### FactoryBot (spec/factories/)

- `medicines.rb`, `people.rb`
- `medication_takes.rb`, `person_medicines.rb`

Fixtures are loaded via `config.fixture_paths` and used with transactional tests.

---

## Replication Checklist

1. **Add gems** to Gemfile (capybara-playwright-driver, rspec-rails, shoulda-matchers, pundit-matchers, factory_bot_rails)
2. **Add `test` stage** to multi-stage Dockerfile with Playwright dependencies
3. **Add `test` profile** to compose.yaml with PostgreSQL service and healthcheck
4. **Configure spec/rails_helper.rb** with Playwright driver for system tests
5. **Add spec/support/** helpers (capybara.rb, authentication_helpers.rb)
6. **Set up Taskfile** for `task test` command
7. **Split CI jobs** into system vs non-system for faster feedback

---

## Coverage Gate

SimpleCov runs with branch coverage enabled (`.simplecov`) and an **enforced
ratchet**: `minimum_coverage line: 89, branch: 73`. It only fires when
`COVERAGE=true` (CI's non-system job), so local single-file runs are unaffected.
If coverage drops below the threshold the build fails. Raise the numbers as
coverage improves; never lower them without a recorded reason.

```bash
# Measure coverage locally (non-system suite):
task test:exec CMD='env COVERAGE=true bundle exec rspec --tag ~browser'
task test:exec CMD='cat coverage/.last_run.json'
```

---

## Mutation Testing

We use [mutant](https://github.com/mbj/mutant) (`usage: opensource` — free for
this public repo, **no licence/token**; config in `config/mutant.yml`). Mutant
mutates the source and checks your specs actually fail — surviving mutants are
assertions you're missing.

**Requirement:** specs must `RSpec.describe TheConstant` (the class constant, not
a string) so mutant can auto-select the covering specs.

```bash
task test:up                                   # ensure the test DB is running
task mutation SUBJECT=MedicationFriendlyName   # mutate one subject
task mutation SUBJECT='GlobalSearch::ResultBuilder*'   # quote :: and *
task mutation:since                            # only subjects changed since origin/main
task mutation:since REF=HEAD~3
```

Notes:
- **Day-to-day, use `task mutation:since`** (or the advisory CI `mutation` job) —
  it only mutates changed subjects, so it's fast. Full per-class runs are a
  one-off baseline activity, and DB-backed subjects (policies/models) are slow
  (every mutation hits PostgreSQL).
- Mutant exits non-zero when any mutant survives — read the `Coverage:`/`Alive:`
  figures rather than the exit code.
- **Equivalent mutants** (no test can distinguish them without changing app code)
  are expected; document them in the commit rather than chasing 100%. Mutant's
  `ignore:` is method-level (too coarse), so don't use it to fake coverage.
- The `.mutant/` directory is an incremental result cache (gitignored).
- CI runs a non-blocking `mutation` job (`mutant run --since origin/main`).
