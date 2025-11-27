---
trigger: always_on
---

# Tooling

- **Language**: Ruby 3.4
- **Framework**: Ruby on Rails 8.0
- **Testing**: RSpec 8.0 + Capybara
- **Test Data**: Standard Rails Fixtures
- **API Mocking**: VCR
- **Code Style**: RuboCop
- **Static Assets**: Propshaft

Use Taskfiles to run commands:
run `task --list` for up to date commands if not sure

task: Available tasks for this project:

- dev-logs:           View development server logs
- dev-rebuild:        Rebuild development server
- dev-seed:           Seed development database with fixtures
- dev-stop:           Stop development server
- dev-up:             Start development server
- test:               Run tests in Docker (optionally specify TEST_FILE=path/to/spec)
- test-logs:          View test server logs
- test-rebuild:       Rebuild test server
- test-seed:          Seed test database with fixtures
- test-stop:          Stop test server
