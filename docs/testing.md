# Testing Guide

MedTracker includes comprehensive testing infrastructure to ensure code quality, performance, and accessibility.

## Test Framework

The project uses **RSpec** with **Capybara** for system tests:

- **RSpec**: Behavior-driven testing framework
- **Capybara**: Integration testing for web applications
- **VCR**: Record and replay HTTP interactions

## Running Tests

### Full Test Suite

Run all tests:

```bash
bundle exec rspec
```

### Specific Test Types

Run only specific types of tests:

```bash
# Model tests
bundle exec rspec spec/models

# Controller tests
bundle exec rspec spec/controllers

# System/Feature tests
bundle exec rspec spec/features

# Policy tests
bundle exec rspec spec/policies
```

### Individual Test Files

Run a specific test file:

```bash
bundle exec rspec spec/models/prescription_spec.rb
```

### Single Test

Run a specific test by line number:

```bash
bundle exec rspec spec/models/prescription_spec.rb:42
```

## Test Structure

Tests are organized in the `spec/` directory:

```
spec/
├── models/           # Model tests
├── controllers/      # Controller tests
├── features/         # System/integration tests
├── policies/         # Authorization policy tests
├── components/       # Phlex component tests
├── fixtures/         # Test data
├── support/          # Test helpers
└── rails_helper.rb   # RSpec configuration
```

## Writing Tests

### Test-Driven Development (TDD)

MedTracker follows the Red-Green-Refactor cycle:

1. **Red**: Write a failing test
2. **Green**: Write minimal code to make it pass
3. **Refactor**: Clean up while keeping tests green

### Model Tests

Example model test:

```ruby
require 'rails_helper'

RSpec.describe Prescription do
  describe 'validations' do
    it 'requires a medicine' do
      prescription = Prescription.new(medicine: nil)
      expect(prescription).not_to be_valid
      expect(prescription.errors[:medicine]).to include("can't be blank")
    end
  end

  describe '#active?' do
    it 'returns true when prescription is within date range' do
      prescription = Prescription.create!(
        medicine: create(:medicine),
        start_date: 1.day.ago,
        end_date: 1.day.from_now
      )
      expect(prescription.active?).to be true
    end
  end
end
```

### System Tests

Example system test with Capybara:

```ruby
require 'rails_helper'

RSpec.describe 'Prescriptions', type: :feature do
  it 'allows creating a new prescription' do
    medicine = create(:medicine)
    
    visit new_prescription_path
    
    select medicine.name, from: 'Medicine'
    fill_in 'Dosage', with: '500mg'
    click_button 'Create Prescription'
    
    expect(page).to have_content('Prescription created successfully')
    expect(page).to have_content('500mg')
  end
end
```

### Policy Tests

Example authorization test:

```ruby
require 'rails_helper'
require 'pundit/rspec'

RSpec.describe PrescriptionPolicy do
  subject(:policy) { described_class.new(user, prescription) }

  let(:prescription) { create(:prescription) }

  context 'when user is an administrator' do
    let(:user) { create(:user, role: 'administrator') }
    
    it { is_expected.to permit_action(:update) }
    it { is_expected.to permit_action(:destroy) }
  end
end
```

## Test Data

### Fixtures

Test data is managed using Rails fixtures in `spec/fixtures/`:

```yaml
# spec/fixtures/medicines.yml
ibuprofen:
  name: Ibuprofen
  generic_name: Ibuprofen
  strength: 200mg
  form: Tablet

paracetamol:
  name: Paracetamol
  generic_name: Acetaminophen
  strength: 500mg
  form: Tablet
```

### Factory Pattern

When fixtures aren't sufficient, use the factory pattern:

```ruby
# In tests
let(:medicine) { Medicine.create!(name: 'Aspirin', strength: '100mg') }
```

## Test Coverage

### Running with Coverage

Generate a coverage report:

```bash
COVERAGE=true bundle exec rspec
```

Coverage reports are generated in `coverage/` directory. Open `coverage/index.html` to view detailed coverage information.

### Coverage Goals

- **Overall Coverage**: Aim for >80%
- **Models**: Aim for >90%
- **Critical Paths**: Aim for 100%

## Continuous Integration

Tests run automatically on every pull request via GitHub Actions.

### CI Pipeline

1. **Lint**: RuboCop checks code style
2. **Security**: Brakeman scans for vulnerabilities
3. **Tests**: Full RSpec suite
4. **Lighthouse**: Performance and accessibility audits

View the CI configuration in `.github/workflows/ci.yml`.

## Lighthouse Audits

MedTracker includes automated Lighthouse audits for performance, accessibility, and best practices.

### Running Locally

First, start the development server:

```bash
task dev:up
```

Then run Lighthouse:

```bash
# Run audit on dashboard
task lighthouse:run

# Run on a specific page
task lighthouse:run URL=http://localhost:3000/medicines

# View score summary
task lighthouse:summary

# List failed audits
task lighthouse:failed-audits
```

### Lighthouse Thresholds

CI enforces the following minimum scores:

| Category | Threshold |
|----------|-----------|
| Performance | 70% |
| Accessibility | 85% |
| Best Practices | 85% |

### Lighthouse Reports

Reports are:
- Automatically generated on every CI run
- Uploaded as artifacts
- Retained for 30 days
- Available in the GitHub Actions interface

### Interpreting Results

Review Lighthouse reports for:

- **Performance**: Page load times, Time to Interactive
- **Accessibility**: ARIA attributes, color contrast, keyboard navigation
- **Best Practices**: HTTPS usage, console errors, image optimization
- **SEO**: Meta tags, structured data

## Docker Testing

Run tests in a containerized environment:

```bash
# Run full test suite
docker compose -f docker-compose.test.yml up --abort-on-container-exit

# Run specific tests
./run test spec/models
```

## Debugging Tests

### Interactive Debugging

Add a breakpoint in your test:

```ruby
require 'debug'

it 'does something' do
  binding.break  # Debugger will pause here
  expect(something).to be_truthy
end
```

### Screenshot Debugging

For system tests, capture screenshots on failure:

```ruby
it 'shows the form', :js do
  visit new_prescription_path
  # Test fails, screenshot saved automatically
end
```

Screenshots are saved to `tmp/capybara/`.

### Database Inspection

Inspect the database during tests:

```ruby
it 'creates a record' do
  # Pause test to inspect database
  binding.break
  # Check data with: Prescription.all
end
```

## Performance Testing

### Profiling

Profile slow tests:

```bash
bundle exec rspec --profile 10
```

This shows the 10 slowest tests.

### Database Queries

Monitor database queries in tests:

```ruby
it 'performs efficiently' do
  expect {
    Prescription.active.includes(:medicine).to_a
  }.to make_database_queries(count: 2)
end
```

## Best Practices

1. **Descriptive Names**: Use clear test descriptions
2. **One Assertion**: Focus tests on single behaviors
3. **Arrange-Act-Assert**: Structure tests clearly
4. **Test Behavior**: Test outcomes, not implementation
5. **Use Fixtures**: Leverage shared test data
6. **Mock External Services**: Use VCR for HTTP calls
7. **Clean Database**: Tests should not depend on order
8. **Fast Tests**: Keep tests fast with proper setup

## Common Issues

### Flaky Tests

If tests fail intermittently:

1. Check for timing issues in system tests
2. Ensure database is properly cleaned between tests
3. Look for order-dependent tests

### Slow Tests

If tests are slow:

1. Use database transactions instead of truncation
2. Minimize system test usage
3. Stub external HTTP calls
4. Profile and optimize slow tests

### Database Pollution

If tests interfere with each other:

1. Ensure DatabaseCleaner is configured
2. Use database transactions
3. Reset database state in `before` hooks

## Resources

- [RSpec Documentation](https://rspec.info/)
- [Capybara Documentation](https://github.com/teamcapybara/capybara)
- [Testing Rails Applications Guide](https://guides.rubyonrails.org/testing.html)
- [VCR Documentation](https://github.com/vcr/vcr)
