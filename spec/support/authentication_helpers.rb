# frozen_string_literal: true

module AuthenticationHelpers
  # Helper method to login via Rodauth
  def rodauth_login(email, password = 'password')
    visit '/login'
    fill_in 'Email address', with: email
    fill_in 'Password', with: password
    click_button 'Login'

    # Wait for login to complete
    using_wait_time(3) do
      expect(page).to have_current_path('/dashboard')
    end
  end

  # Helper method to login using a user fixture
  def login_as(user)
    rodauth_login(user.email_address)
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
