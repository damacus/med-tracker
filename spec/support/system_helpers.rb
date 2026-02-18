# frozen_string_literal: true

# This module contains helper methods for system tests, using the Capybara DSL.
module SystemHelpers
  # Signs in a user using the login form.
  # This helper uses standard Capybara methods to interact with the page.
  # Clears any 2FA setup to allow direct login without TOTP.
  def sign_in(user)
    # Clear 2FA to allow direct login
    account = user.respond_to?(:person) ? user.person.account : user
    clear_2fa_for_account(account) if account.respond_to?(:id)

    visit '/login'

    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: 'password'

    click_button 'Sign In to Dashboard'

    expect(page).to have_current_path('/dashboard')
  end
end

RSpec.configure do |config|
  # Include these helpers in all system tests.
  config.include SystemHelpers, type: :system
  config.include SystemHelpers, type: :feature
end
