# frozen_string_literal: true

module AuthenticationHelpers
  # Helper method to login via Rodauth
  def rodauth_login(email, password = 'password')
    expected_path = expected_dashboard_path_for(email)

    visit '/login'
    fill_in 'Email address', with: email
    fill_in 'Password', with: password
    click_button 'Sign In to Dashboard'

    using_wait_time(3) do
      expect(page).to have_current_path(expected_path)
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
    household_user = household_user_for(user)
    household = ensure_api_household_for(household_user) if household_user
    @browser_household = household if household
    @browser_membership = browser_membership_for(household_user, household) if household_user && household

    account = account_for_authentication(user)
    clear_2fa_for_account(account) if account.respond_to?(:id)

    email = user.respond_to?(:email_address) ? user.email_address : user.email
    rodauth_login(email)
  end

  # Helper method for logout
  def rodauth_logout
    visit '/logout'
  end

  def expected_dashboard_path_for(email)
    household = @browser_household || account_household_for(email)

    return %r{\A/households/[^/]+/dashboard\z} unless household

    "/households/#{household.slug}/dashboard"
  end

  def account_household_for(email)
    account = Account.find_by(email: email)
    return unless account

    account.first_active_household || account.person&.household
  end

  def account_for_authentication(user)
    return user.person.account if user.respond_to?(:person) && user.person&.account

    user
  end

  def household_user_for(user)
    return user if user.respond_to?(:email_address) && user.respond_to?(:person)
    return user.person.user if user.respond_to?(:person) && user.person&.user

    nil
  end

  def browser_membership_for(user, household)
    household.household_memberships.active.find_by(account: user.person.account)
  end
end

RSpec.configure do |config|
  config.include AuthenticationHelpers, type: :system
  config.include AuthenticationHelpers, type: :feature
end
