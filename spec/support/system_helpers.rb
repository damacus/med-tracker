# frozen_string_literal: true

# This module contains helper methods for system tests, using the Capybara DSL.
module SystemHelpers
  # Signs in a user using the login form.
  # This helper uses standard Capybara methods to interact with the page.
  def sign_in(user)
    visit '/login'

    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: 'password'

    click_button 'Login'

    expect(page).to have_current_path('/dashboard')
  end
end

RSpec.configure do |config|
  # Include these helpers in all system tests.
  config.include SystemHelpers, type: :system
  config.include SystemHelpers, type: :feature
end
