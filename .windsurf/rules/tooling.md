---
trigger: always_on
---

# Tooling

- **Language**: Ruby
- **Framework**: Ruby on Rails
- **Testing**: RSpec + Capybara
- **Test Data**: Standard Rails Fixtures
- **API Mocking**: VCR
- **Code Style**: RuboCop
- **Static Assets**: Propshaft

Use Taskfiles to run commands:
run task --list for up to date commands if not sure

task: Available tasks for this project:
* compose:                Run RSpecs tests using Docker Compose
* compose:dev:            Run Rails server using Docker Compose
* compose:dev:logs:       Follow logs for Rails server using Docker Compose
* compose:dev:stop:       Stop Rails server using Docker Compose
* local:rubocop:          Run Rubocop tests (installs Playwright, sets up DB, runs RSpec)
* local:test:             Run Playwright tests (installs Playwright, sets up DB, runs RSpec)
