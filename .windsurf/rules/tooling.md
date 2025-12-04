---
trigger: model_decision
description: When running commands, setting up environment, or using project tools
---

# Tooling

## Stack

- **Language**: Ruby 3.4
- **Framework**: Ruby on Rails 8.1
- **Database**: PostgreSQL 18 (all environments)
- **Testing**: RSpec + Capybara
- **Test Data**: Rails Fixtures
- **Code Style**: RuboCop
- **Views**: Phlex components + Hotwire (Turbo/Stimulus)
- **Static Assets**: Propshaft

## Task Commands

Always use Taskfile commands (run `task --list` for current list):

| Command         | Purpose                     |
|-----------------|-----------------------------|
| `task test`     | Run tests in Docker         |
| `task dev-up`   | Start development server    |
| `task dev-seed` | Seed database with fixtures |
| `task dev-stop` | Stop development server     |
| `task dev-logs` | View development logs       |

## Shell

- Use **fish shell** syntax for all commands
- Never use `cd` in run_command; use `cwd` parameter instead

## Git

- Conventional Commits format (`feat:`, `fix:`, `refactor:`, `test:`)
- Small, atomic commits
- All tests must pass before push
