# GitHub Copilot Instructions for MedTracker

## Mission
- **Overall goal** Manage and track medication schedules so carers and patients can confidently record doses and respect safety rules.
- **Primary outcome** Accurate, auditable history of prescriptions and over-the-counter medicines for every `Person`.
- **Source of truth** Domain logic lives on the Rails server; the front end renders server-sent HTML via Hotwire.

## Project Overview

MedTracker is a Ruby on Rails application for managing and tracking medication schedules. It helps users monitor medication intake, ensuring adherence to prescribed schedules with built-in validations to prevent overdose and timing violations.

## Orientation
- **Read first** `README.md` for setup, `docs/design.md` for architecture, `USER_MANAGEMENT_PLAN.md` for roadmap.
- **Key directories** `app/` (Rails MVC + Phlex components), `spec/` (RSpec and Capybara tests), `config/` (environment + routes), `db/migrate/` (schema history).
- **Named guides** `PERSON_MEDICINES_IMPLEMENTATION.md` documents the non-prescription flow.

## Tech Stack

- **Language**: Ruby 3.4.7
- **Framework**: Ruby on Rails 8.0+
- **Database**: SQLite3 (development)
- **Frontend**: Hotwire (Turbo, Stimulus), Phlex views, TailwindCSS
- **Testing**: RSpec + Capybara (for system tests)
- **Test Data**: Standard Rails Fixtures
- **API Mocking**: VCR
- **Code Style**: RuboCop (standard configuration)
- **Static Assets**: Propshaft

## Domain Model Highlights
- **People & Users** `Person` stores demographic data; `User` handles authentication and roles (administrator, doctor, nurse, carer, parent).
- **Medicines** `Medicine` plus `Prescription` define formal regimens; `PersonMedicine` covers ad-hoc supplements; `MedicationTake` logs every dose.
- **Relationships** `CarerRelationship` links carers to patients, enforcing capacity support.
- **Constraints** Timing rules (`max_daily_doses`, `min_hours_between_doses`) enforced in models before UI.

## Application Design
- **Backend** Ruby on Rails with service-style POROs when business logic grows; guard clauses preferred for early exits.
- **Frontend** Hotwire (Turbo + Stimulus) with Phlex components under `app/components/`; dialogs lean on HTML `<dialog>` and Turbo Streams.
- **Authentication** Cookie sessions, `has_secure_password`, IP and user-agent tracking via `Authentication` concern; future OIDC planned (`docs/design.md`).
- **Admin area** Namespaced controllers (`app/controllers/admin/`) and Phlex views manage users.

## Development Principles

### Test-Driven Development (TDD)

Always follow the Red-Green-Refactor cycle:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Clean up the code while keeping tests green

**Critical**: No production code without a failing test. This discipline ensures every feature is properly tested and prevents regressions.

### Testing Guidelines

- Use **RSpec** as the testing framework (`_spec.rb` files)
- Write behavior-driven tests that verify expected outcomes, not implementation details
- Use Capybara for end-to-end user interaction tests (system specs)
- Test through public APIs only (controller actions, public model methods)
- Use standard Rails fixtures for test data (located in `spec/fixtures/`)
- Fixtures must be well-organized, realistic, and unique
- Use VCR to record and replay HTTP interactions with external services (store cassettes in `spec/vcr_cassettes`)
- Follow Rails/RSpec conventions for test file locations (`spec/models`, `spec/features`, etc.)
- Tests should document expected business behavior
- Aim for exhaustive coverage of policy, model, and feature behavior

Example test structure:
```ruby
require 'rails_helper'
require 'pundit/rspec'

RSpec.describe UserPolicy do
  subject(:policy) { described_class.new(current_user, user) }

  let(:user) { User.new(person: Person.new(name: 'Test User', date_of_birth: 20.years.ago)) }

  context 'when user is an administrator' do
    let(:current_user) { User.new(person: Person.new(name: 'Admin', date_of_birth: 30.years.ago), role: 'administrator') }
    
    it { is_expected.to permit_action(:update) }
  end
end
```

### Code Style

- **Style Guide**: Adhere to the community Ruby Style Guide, enforced by the standard RuboCop configuration.
- **Self-Documenting Code**: Write clear, self-documenting code. Avoid comments; use descriptive names and extract complex logic into well-named private methods.
- **Functional-Style Ruby**: Prefer Enumerable methods (`map`, `select`, `reduce`) over imperative loops.
- Use guard clauses instead of nested if/else statements
- Keep methods small and focused (Single Responsibility Principle)
- Avoid deep nesting (max 2 levels)
- Choose descriptive names for variables, methods, and classes

#### Naming Conventions

- Methods/Variables: `snake_case`
- Classes/Modules: `PascalCase`
- Constants: `UPPER_SNAKE_CASE`
- Files: `snake_case.rb`
- Test files: `*_spec.rb` (RSpec convention)

### Ruby and Rails Best Practices

- Use Active Record validations in models for data integrity
- Use Strong Parameters in controllers to prevent mass assignment
- Strive for immutability where practical
- Prefer flat, readable code over clever one-liners
- Extract complex logic into well-named private methods
- **Service Objects**: For complex business logic that doesn't fit in a model or controller, use Plain Old Ruby Objects (POROs) or Service Objects.
- **Architecture**: Use service objects for complex logic, avoid deep controller/model coupling
- **Typing & safety**: Maintain existing type hints; never loosen types to appease linters

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
- Non-prescription/ad-hoc medicine tracking via `PersonMedicine`
- Carer relationships with capacity support tracking

## Useful Entry Points
- **Authentication** `app/controllers/sessions_controller.rb`, `spec/features/sessions_spec.rb`.
- **Medication tracking** `app/models/medication_take.rb`, `spec/features/person_medicines_spec.rb`.
- **Admin users** `app/controllers/admin/users_controller.rb`, `spec/components/admin/`.

## Tooling & Automation
- **Workflows** Review `.windsurf/workflows/` for task-specific playbooks (`/test`, `/rubocop`, `/update-dependencies`, etc.).
- **Linting** RuboCop config lives in `.rubocop.yml`; respect enforced cops.
- **CI** GitHub Actions (`.github/workflows/`) run tests and Playwright suites.

## Commit Message Guidelines

Use semantic commits (conventional commits format) for all commit messages:

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, missing semi-colons, etc.)
- `refactor:` - Code refactoring without changing functionality
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks, dependency updates

**Examples:**
```
feat: add prescription management feature
fix: resolve dose timing validation error
docs: update README with setup instructions
refactor: extract dosage calculation to service object
test: add tests for medication take model
```

**Important:**
- Do NOT create "Initial plan" or "Initial commit" messages as they pollute git history
- Start with meaningful, descriptive commits that reflect actual work completed
- Each commit should represent a complete, logical unit of work
- Small atomic changes, always green tests before merge

## Running Commands

### Setup
```bash
bundle install
rails db:create db:migrate
```

### Testing
```bash
bundle exec rspec               # Run all tests
rails test                      # Alternative (if Minitest tests exist)
```

### Code Quality
```bash
rubocop                        # Check code style
rubocop -a                     # Auto-fix style issues
brakeman                       # Security analysis
```

### Development Server
```bash
bin/dev                        # Start server (recommended)
rails server                   # Alternative: Start server at http://localhost:3000
```

## Important Notes

- **No comments in code** - write self-documenting code instead. Do not add or remove comments unless explicitly told by the user.
- **Always write tests before implementing features** - strict Red-Green-Refactor workflow
- Keep documentation (README, CONTRIBUTING) up to date with changes
- Authorization is handled by Pundit - check policies before modifying access control
- The project uses Phlex for views, not ERB templates
- Environment: Run rails server via `bin/dev`; avoid destructive commands without confirmation
- Documentation: Markdown must satisfy `markdown-lint-cli2`; keep headings orderly

## Future Plans
- **Roadmap** `USER_MANAGEMENT_PLAN.md` phases detail authorization, admin CRUD, carer tools, and audit logging.
- **Design vision** `docs/design.md` notes eventual OIDC auth, audit trails, mobile support.
- **Open tasks** Look for unchecked items in plan documents before adding new features.
