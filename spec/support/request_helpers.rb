# frozen_string_literal: true

# Helper methods for request specs (non-Capybara HTTP tests)
module RequestHelpers
  # Signs in a user via Rodauth for request specs.
  # Uses direct HTTP POST instead of Capybara page interactions.
  # Clears any 2FA setup to allow direct login without TOTP.
  def sign_in(user)
    account = Account.find_by(email: user.email_address)

    # Clear 2FA to allow direct login
    AccountOtpKey.where(id: account.id).delete_all
    AccountRecoveryCode.where(id: account.id).delete_all
    account.account_webauthn_keys.destroy_all if account.respond_to?(:account_webauthn_keys)

    post '/login', params: { email: account.email, password: 'password' }
    follow_redirect! if response.redirect?
  end
end

RSpec.configure do |config|
  config.include RequestHelpers, type: :request
end
