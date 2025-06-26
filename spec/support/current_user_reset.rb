# frozen_string_literal: true

# Reset Current.session before each system test to prevent test pollution
RSpec.configure do |config|
  config.before(:each, type: :system) do
    # Reset Current.session to ensure clean user authentication state between tests
    Current.session = nil
  end
end
