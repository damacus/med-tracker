# GitHub Copilot Instructions for MedTracker

## Project Overview

MedTracker is a Ruby on Rails application for managing and tracking medication schedules. It helps users monitor medication intake, ensuring adherence to prescribed schedules with built-in validations to prevent overdose and timing violations.

## Tech Stack

- **Language**: Ruby 3.4.7
- **Framework**: Ruby on Rails 8.0+
- **Database**: SQLite3 (development)
- **Frontend**: Hotwire (Turbo, Stimulus), Phlex views, TailwindCSS
- **Testing**: Minitest + Capybara (for system tests)
- **Code Style**: RuboCop (Rails Omakase)

## Development Principles

### Test-Driven Development (TDD)

Always follow the Red-Green-Refactor cycle:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Clean up the code while keeping tests green

### Testing Guidelines

- Use **Minitest** as the testing framework (not RSpec)
- Write behavior-driven tests that verify expected outcomes, not implementation details
- Test through public APIs only (controller actions, public model methods)
- Use Rails fixtures for test data (located in `test/fixtures/`)
- System tests use Capybara to simulate user interactions
- Tests should document expected business behavior
- Aim for 100% test coverage through behavior testing

Example test structure:
```ruby
require "test_helper"

class UserTest < ActiveSupport::TestCase
  fixtures :users

  test "returns the user's name" do
    assert_equal "David Heinemeier Hansson", users(:david).name
  end
end
```

### Code Style

- Follow RuboCop Rails Omakase guidelines
- Use guard clauses instead of nested if/else statements
- Keep methods small and focused (Single Responsibility Principle)
- Avoid deep nesting (max 2 levels)
- Write self-documenting code - avoid comments in code
- Choose descriptive names for variables, methods, and classes

#### Naming Conventions

- Methods/Variables: `snake_case`
- Classes/Modules: `PascalCase`
- Constants: `UPPER_SNAKE_CASE`
- Files: `snake_case.rb`
- Test files: `*_test.rb` (Minitest convention)

### Ruby and Rails Best Practices

- Use Active Record validations in models for data integrity
- Use Strong Parameters in controllers to prevent mass assignment
- Strive for immutability where practical
- Prefer flat, readable code over clever one-liners
- Extract complex logic into well-named private methods

### Data Validation

- Always use Active Record validations in models
- Use Strong Parameters in controllers
- Validate business rules at the model level

## Project Structure

```
app/
  assets/           # Static assets
  channels/         # ActionCable channels
  components/       # Phlex components
  controllers/      # Rails controllers
  helpers/          # View helpers
  javascript/       # Stimulus controllers
  jobs/            # ActiveJob jobs
  mailers/         # ActionMailer mailers
  models/          # ActiveRecord models
  policies/        # Pundit authorization policies
  services/        # Service objects
  views/           # Phlex views
```

## Key Features

- Prescription management with dosage tracking
- Dose timing restrictions (max daily doses, minimum hours between doses)
- Support for daily, weekly, and monthly dosing cycles
- Active/inactive prescription tracking
- Smart validations to prevent medication errors
- Authorization using Pundit
- Passkey authentication support

## Running Commands

### Setup
```bash
bundle install
rails db:create db:migrate
```

### Testing
```bash
bundle exec rake test        # Run all tests
rails test                   # Alternative test runner
rails test:system           # Run system tests only
```

### Code Quality
```bash
rubocop                     # Check code style
rubocop -a                  # Auto-fix style issues
brakeman                    # Security analysis
```

### Development Server
```bash
rails server                # Start server at http://localhost:3000
```

## Important Notes

- No comments in code - write self-documenting code instead
- Always write tests before implementing features
- Keep documentation (README, CONTRIBUTING) up to date with changes
- Authorization is handled by Pundit - check policies before modifying access control
- The project uses Phlex for views, not ERB templates
