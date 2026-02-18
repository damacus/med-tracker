# frozen_string_literal: true

module AuthenticationHelpers
  # Helper method to login via Rodauth
  def rodauth_login(email, password = 'password')
    visit '/login'
    fill_in 'Email address', with: email
    fill_in 'Password', with: password
    click_button 'Sign In to Dashboard'

    # Wait for login to complete
    using_wait_time(3) do
      expect(page).to have_current_path('/dashboard')
    end
  end

  # Complete TOTP authentication using the test secret
  def complete_totp_auth(secret = 'JBSWY3DPEHPK3PXP')
    totp = ROTP::TOTP.new(secret)
    fill_in 'Authentication code', with: totp.at(Time.current)
    click_button 'Verify code'
  end

  # Helper method to login using a user fixture or account
  # Clears any 2FA setup to allow direct login without TOTP
  def login_as(user)
    account = user.respond_to?(:person) ? user.person.account : user
    clear_2fa_for_account(account) if account.respond_to?(:id)

    email = user.respond_to?(:email_address) ? user.email_address : user.email
    rodauth_login(email)
  end

  # Helper method for logout
  def rodauth_logout
    visit '/logout'
  end
end

RSpec.configure do |config|
  config.include AuthenticationHelpers, type: :system
  config.include AuthenticationHelpers, type: :feature
end
