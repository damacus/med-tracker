# frozen_string_literal: true

require 'rails_helper'

# Following the Red-Green-Refactor TDD cycle
# This spec tests the navigation component through system testing
RSpec.describe 'Navigation', type: :system do
  fixtures :users

  before do
    driven_by(:playwright)
  end

  context 'when user is authenticated' do
    it 'renders navigation with sign out button' do
      # Log in user
      user = users(:john) # Admin user from fixtures

      visit login_path
      fill_in 'Email address', with: user.email_address
      fill_in 'Password', with: 'password'
      click_button 'Login'

      # Check navigation elements for authenticated user
      within 'nav' do
        expect(page).to have_link('Medicines')
        expect(page).to have_link('People')
        expect(page).to have_no_link('Login')

        # Open the user dropdown menu to access Logout link
        click_button user.name
      end

      # Dropdown content may render outside nav, check page-wide
      expect(page).to have_link('Logout')
    end
  end

  context 'when user is not authenticated' do
    it 'renders navigation with login link' do
      # Ensure we start with a fresh session
      driven_by(:playwright) # Re-initialize driver to reset session state
      Capybara.reset_sessions!

      # First try to log out explicitly if we're logged in
      visit root_path
      click_link 'Logout' if page.has_link?('Logout')

      # Then ensure we visit a page as a guest
      visit login_path

      # Check navigation elements for unauthenticated user
      within 'nav' do
        expect(page).to have_link('Login')
        expect(page).to have_no_link('Logout')
        expect(page).to have_no_link('Medicines')
        expect(page).to have_no_link('People')
      end
    end
  end
end
