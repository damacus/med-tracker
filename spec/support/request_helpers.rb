# frozen_string_literal: true

# Helper methods for request specs (non-Capybara HTTP tests)
module RequestHelpers
  # Signs in a user via Rodauth for request specs.
  # Uses direct HTTP POST instead of Capybara page interactions.
  def sign_in(user)
    account = Account.find_by(email: user.email_address)
    post '/login', params: { email: account.email, password: 'password' }
    follow_redirect! if response.redirect?
  end
end

RSpec.configure do |config|
  config.include RequestHelpers, type: :request
end
