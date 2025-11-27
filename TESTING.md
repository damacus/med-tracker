# MedTracker Test Setup Guide

## Overview

MedTracker uses a **Dockerized PostgreSQL-based test environment** with RSpec, Capybara, and Playwright for browser automation. Tests run in containers to ensure dev/test/prod parity.

---

## Core Stack

| Component       | Tool                          | Version   |
|-----------------|-------------------------------|-----------|
| Language        | Ruby                          | 3.4.7     |
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

### docker-compose.test.yml

```yaml
services:
  db:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: medtracker_test
      POSTGRES_PASSWORD: medtracker_test_password
      POSTGRES_DB: medtracker_test
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U medtracker_test"]
      interval: 5s
      timeout: 3s
      retries: 5

  web:
    build:
      context: .
      dockerfile: Dockerfile.test
    depends_on:
      db:
        condition: service_healthy
    environment:
      RAILS_ENV: test
      DATABASE_URL: postgres://medtracker_test:medtracker_test_password@db:5432/medtracker_test
    volumes:
      - .:/app
      - bundle_cache:/usr/local/bundle
      - ./tmp/capybara:/app/tmp/capybara
```

### Dockerfile.test (key sections)

```dockerfile
FROM ruby:3.4.7-slim AS test-assets

# Install Playwright dependencies
RUN apt-get install -y libnss3 libatk-bridge2.0-0 libdrm2 libxkbcommon0 \
    libxcomposite1 libxdamage1 libxrandr2 libgbm1 libxss1 libasound2

# Install Playwright browsers
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
RUN npx playwright install --with-deps chromium && chmod -R 755 /ms-playwright

CMD ["bundle", "exec", "rspec"]
```

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
2. **Create Dockerfile.test** with Playwright dependencies and browser installation
3. **Create docker-compose.test.yml** with PostgreSQL service and healthcheck
4. **Configure spec/rails_helper.rb** with Playwright driver for system tests
5. **Add spec/support/** helpers (capybara.rb, authentication_helpers.rb)
6. **Set up Taskfile** for `task test` command
7. **Split CI jobs** into system vs non-system for faster feedback
