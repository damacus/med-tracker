# Ruby & Rails Code Style Guide

## General Principles
- **Modern Ruby:** Use Ruby 4.0+ features where appropriate (e.g., pattern matching, shorthand hash syntax).
- **Expressive & Concise:** Favor `Enumerable` methods (`map`, `select`, `reduce`) over imperative loops.
- **Guard Clauses:** Use early returns to reduce nesting and improve readability.

## Rails 8.1 Conventions
- **Solid Suite:** Utilize Solid Queue and Solid Cache for background jobs and caching.
- **Phlex Components:** All views must be Phlex components in `app/components/`. 
- **STRICT:** Do NOT use ERB files.
- **Controllers:** Keep controllers thin; delegate business logic to models or service objects.
- **Models:** Use Active Record validations and associations. Prefer `delegated_type` for polymorphic relationships.

## Naming Conventions
- **Classes/Modules:** PascalCase (e.g., `UserAuthenticator`).
- **Methods/Variables:** snake_case (e.g., `calculate_dose`).
- **Predicates:** End methods returning booleans with a `?` (e.g., `authorized?`).
- **Dangerous Methods:** End methods that modify state or raise errors unexpectedly with `!` (e.g., `save!`).

## Phlex & UI
- **Components:** Organize UI elements into reusable Phlex components in `app/components/`.
- **Helpers:** Prefer component-local methods over global Rails helpers for view logic.
- **RubyUI:** Adhere to established patterns in the `app/components/ruby_ui/` directory.

## Testing (RSpec)
- **Descriptive Specs:** Use `describe` for classes/methods and `context` for state-specific scenarios.
- **Subject/Let:** Use `let` and `subject` for cleaner setup.
- **Fixtures:** Use standard Rails fixtures for test data as per project convention.
