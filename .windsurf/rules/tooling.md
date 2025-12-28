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
- **Views**: RubyUI (Phlex) + Hotwire
  - **STRICT:** All views must be `.rb` files.
  - **FORBIDDEN:** Do NOT create `.erb` files.
- **Static Assets**: Propshaft

## Data Inspection (Strict)

- **JSON**: Always use `jq` to search or filter JSON files.
- **Method**: Run `jq` commands directly in the shell.
- **Forbidden**: Do not write Ruby/Python scripts to parse JSON. Do not read raw JSON text.

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
