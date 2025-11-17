# frozen_string_literal: true

# This module contains helper methods for system tests, using the Capybara DSL.
module SystemHelpers
  # Signs in a user using the login form.
  # This helper uses standard Capybara methods to interact with the page.
  # It also sets Current.session to ensure the Navigation component can detect the authenticated state.
  def sign_in(user)
    visit '/login'

    fill_in 'email_address', with: user.email_address
    fill_in 'password', with: 'password'

    click_button 'Login'

    expect(page).to have_current_path('/')

    Current.session = { user_id: user.id }
  end
end

RSpec.configure do |config|
  # Include these helpers in all system tests.
  config.include SystemHelpers, type: :system
end
