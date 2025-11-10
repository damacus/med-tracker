# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Navigation' do
  fixtures :users

  before do
    driven_by(:rack_test)
  end

  context 'when user is authenticated' do
    it 'shows navigation with a sign out button' do
      user = users(:john) # Admin user from fixtures

      visit login_path
      fill_in 'email_address', with: user.email_address
      fill_in 'password', with: 'password'
      click_button 'Sign in'

      # Check navigation elements for authenticated user
      within 'nav' do
        aggregate_failures 'navigation links and buttons' do
          expect(page).to have_link('Medicines')
          expect(page).to have_link('People')
          expect(page).to have_button('Logout')
          expect(page).to have_no_link('Login')
        end
      end
    end
  end

  context 'when user is not authenticated' do
    it 'shows navigation with a login link' do
      # Reset the session and ensure we're logged out
      Capybara.reset_sessions!
      Current.session = nil

      # Use the login page which should always show unauthenticated navigation
      visit login_path

      # Check navigation elements for unauthenticated user
      within 'nav' do
        aggregate_failures 'navigation links and buttons' do
          expect(page).to have_link('Login')
          expect(page).to have_no_button('Logout')
          expect(page).to have_no_link('Medicines')
          expect(page).to have_no_link('People')
        end
      end
    end
  end
end
