# Copilot Instructions for MedTracker

## Project Overview

MedTracker is a Ruby on Rails application designed to help users manage and track their medication schedules. The application ensures users adhere to their prescribed medication schedules and restrictions, preventing common mistakes like taking too much medication or taking doses too close together.

## Tech Stack

- **Framework**: Ruby on Rails 8+
- **Language**: Ruby 3.4.7
- **Database**: SQLite3 (development), PostgreSQL (production via Kamal)
- **Frontend**: Hotwire (Turbo + Stimulus), Phlex components, Tailwind CSS
- **Testing**: RSpec and Minitest (dual testing frameworks) with Capybara for system tests
- **Authorization**: Pundit
- **Authentication**: Passkeys Rails, bcrypt

## Key Models

- **User**: Application users with authentication
- **Person**: People who take medications (can be managed by carers)
- **Medicine**: Medication information (name, dosage forms)
- **PersonMedicine**: Links people to medicines they take
- **Prescription**: Medication prescriptions with dosage and frequency
- **MedicationTake**: Records of doses taken
- **CarerRelationship**: Manages carer-to-person relationships

## Development Guidelines

### Testing Principles

1. **Test-Driven Development (TDD)**
   - Write tests first following the Red-Green-Refactor cycle
   - Tests should verify expected behavior, not implementation details
   - Test through public APIs only (controller actions, public model methods)
   - Aim for 100% test coverage by testing business behavior

2. **Testing Tools**
   - The project uses **both RSpec and Minitest** for testing
   - **Prefer RSpec for new tests** - it's more actively used in this project
   - **RSpec** is used for specs in `spec/` directory (policies, services, components, models, system tests)
   - **Minitest** exists in `test/` directory (some model tests, controllers, system tests)
   - Use **Capybara** for system tests to simulate user interactions
   - Use **VCR** for API mocking
   - Use standard Rails **fixtures** for test data (located in `test/fixtures/` and `spec/fixtures/`)

3. **Test Organization**
   ```
   spec/
     policies/        # Pundit policy specs (RSpec)
     services/        # Service object specs (RSpec)
     components/      # Component specs (RSpec)
     models/          # Model specs (RSpec)
     requests/        # Request specs (RSpec)
     system/          # System specs (RSpec)
   
   test/
     models/          # Model tests (Minitest)
     controllers/     # Controller tests (Minitest)
     system/          # System tests with Capybara (Minitest)
     helpers/         # Helper tests (Minitest)
     integration/     # Integration tests (Minitest)
   ```

### Code Style

1. **Follow RuboCop**: The project uses RuboCop for style enforcement (`.rubocop.yml`)

2. **Naming Conventions**
   - Methods/Variables: `snake_case`
   - Classes/Modules: `PascalCase`
   - Constants: `UPPER_SNAKE_CASE`
   - Files: `snake_case.rb`
   - Test files: `*_test.rb` (Minitest) or `*_spec.rb` (RSpec)

3. **Code Quality**
   - Use guard clauses to avoid nested if/else statements
   - Avoid deep nesting (max 2 levels)
   - Follow Single Responsibility Principle
   - Keep methods small and focused
   - Write self-documenting code; avoid comments

4. **Data Validation**
   - Use Active Record validations in models for data integrity
   - Use Strong Parameters in controllers to prevent mass assignment

5. **Views**
   - Use Phlex for view components (`app/components/`)
   - Components are organized by domain (e.g., `medicines/`, `prescriptions/`)

### Project Structure

- **app/components/**: Phlex view components organized by domain
- **app/controllers/**: Standard Rails controllers with an `admin/` namespace
- **app/models/**: Active Record models
- **app/policies/**: Pundit authorization policies
- **app/services/**: Service objects for complex business logic
- **spec/**: RSpec test files (policies, services, components)
- **test/**: Minitest test files (models, controllers, system tests)

### Development Setup

1. Clone the repository
2. Install dependencies: `bundle install`
3. Set up database: `rails db:create && rails db:migrate`
4. Run tests: `bundle exec rake test` (Minitest) or `bundle exec rspec` (RSpec)
5. Start server: `rails server` (available at http://localhost:3000)

### Important Conventions

1. **No Comments**: Code should be self-documenting through clear naming and structure
2. **Immutability**: Strive for immutability where practical
3. **Small Methods**: Write small, focused methods with single responsibilities
4. **Readability**: Prefer flat, readable code over clever one-liners
5. **Documentation**: Keep project documentation current when introducing meaningful changes

### Testing with Fixtures

Example fixture structure:
```yaml
# test/fixtures/users.yml
admin:
  email: admin@example.com
  password_digest: <%= BCrypt::Password.create('password') %>
```

Use in tests:
```ruby
test "user can log in" do
  user = users(:admin)
  # test logic here
end
```

### Authorization

- Use Pundit policies for authorization
- Policies are located in `app/policies/`
- Each model that requires authorization should have a corresponding policy

### Common Tasks

- **Running tests**: 
  - Minitest: `bundle exec rake test`
  - RSpec: `bundle exec rspec`
- **Linting**: RuboCop is configured (`.rubocop.yml`)
- **Database migrations**: `rails db:migrate`
- **Console**: `rails console`

### Key Business Logic

1. **Dose Tracking**: Validates doses against prescription rules
2. **Timing Restrictions**: Enforces max daily doses and minimum hours between doses
3. **Dose Cycles**: Supports daily, weekly, and monthly dosing schedules
4. **Active/Inactive Prescriptions**: Automatically tracks prescription status based on dates

### Deployment

- Uses Kamal for deployment (`.kamal/` directory)
- Docker configuration available (`Dockerfile`)
- Environment-specific configurations in `config/`
