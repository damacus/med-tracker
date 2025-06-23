# Ruby Development Guidelines

## Quick Reference

**Key Principles:**

- Write tests first (TDD)
- Test behavior, not implementation
- Use Rails' strong parameters for validation.
- Strive for immutability where practical.
- Write small, focused methods.
- Follow the Ruby Style Guide.

**Preferred Tools:**

- **Language**: Ruby
- **Framework**: Ruby on Rails
- **Testing**: Minitest + Capybara
- **Test Data**: Standard Rails Fixtures
- **API Mocking**: VCR
- **Code Style**: RuboCop

## Testing Principles

### Behavior-Driven Testing

- Tests should verify expected behavior, treating implementation as a black box.
- Test through the public API (e.g., controller actions, public model methods) exclusively. Internals should be invisible to tests.
- Tests must document expected business behaviour.
- 100% test coverage is the goal, achieved by testing business behavior, not implementation details.

### Testing Tools

- We use **Minitest**, the default Rails testing framework.
- System tests are written with **Capybara** to simulate user interactions.
- For API mocking, we use **VCR**.

### Test Organization

Rails has a standard directory structure for tests. Our files will be organized as follows:

```text
test/
  models/
    user_test.rb
  controllers/
    users_controller_test.rb
  system/
    user_sign_up_test.rb
```

### Test Data Pattern

We use standard Rails fixtures for test data.

Example with fixtures:

```yaml
# test/fixtures/users.yml
david:
  name: David Heinemeier Hansson
  birthday: 1979-10-15
  profession: Creator of Rails

steve:
  name: Steve Klabnik
  birthday: 1983-06-08
  profession: Ruby Hero
```

And in a test:

```ruby
# test/models/user_test.rb
require "test_helper"

class UserTest < ActiveSupport::TestCase
  fixtures :users

  test "returns the user's name" do
    assert_equal "David Heinemeier Hansson", users(:david).name
  end
end
```

Key principles:

- Fixtures are a way of organizing test data; they reside in the `test/fixtures/` directory.
- Each fixture file corresponds to a model.
- Fixtures are written in YAML.
- They are pre-loaded before each test run, which can be faster than factories.

## Ruby and Rails Guidelines

### Data Validation

- Use Active Record validations in your models to ensure data integrity.
- Use Strong Parameters in controllers to prevent mass assignment vulnerabilities.

- **Use Guard Clauses**: Avoid nested `if/else` statements by using early returns.
- **Avoid Deep Nesting**: Keep methods and blocks shallow (max 2 levels).
- **Single Responsibility Principle**: Keep methods small and focused on a single responsibility.
- **Readability**: Prefer flat, readable code over clever one-liners.

#### Naming Conventions

- **Methods/Variables**: `snake_case` (e.g., `calculate_total`, `user_name`).
- **Classes/Modules**: `PascalCase` (e.g., `PaymentRequest`, `UserProfile`).
- **Constants**: `UPPER_SNAKE_CASE`.
- **Files**: `snake_case.rb`.
- **Test files**: `*_spec.rb` (RSpec).

### No Comments in Code

Code should be self-documenting. Comments often indicate that the code itself is not clear enough and can become outdated.

Instead of comments, focus on:

- Choosing descriptive names for variables, methods, and classes.
- Extracting complex logic into well-named private methods.

#### Good: Self-documenting code with clear names

```ruby
class DiscountCalculator
  PREMIUM_DISCOUNT_MULTIPLIER = 0.8
  STANDARD_DISCOUNT_MULTIPLIER = 0.9

  def self.call(price, customer)
    new(price, customer).calculate
  end

  def initialize(price, customer)
    @price = price
    @customer = customer
  end

  def calculate
    price * discount_multiplier
  end

  private

  attr_reader :price, :customer

  def discount_multiplier
    customer.premium? ? PREMIUM_DISCOUNT_MULTIPLIER : STANDARD_DISCOUNT_MULTIPLIER
  end
end
```

## Refactoring

Follow the **Red-Green-Refactor** cycle of TDD:

1. **Red**: Write a simple test that fails.
2. **Green**: Write the minimum amount of code to make the test pass.
3. **Refactor**: Clean up the code you just wrote, while keeping the tests green.

- **Assess refactoring after every green test** - Look for opportunities to improve code structure.
- **Keep project docs current** - update them whenever you introduce meaningful changes.
