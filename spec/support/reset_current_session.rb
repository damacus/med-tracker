# frozen_string_literal: true

# Reset the Current session before each test to prevent session state leakage between examples
RSpec.configure do |config|
  # Reset Current.session before each example to ensure clean authentication state
  config.before(:each, type: :system) do
    # Explicitly reset the Current session to nil to ensure clean authentication state
    Current.session = nil
  end
end
