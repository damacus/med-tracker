# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Navigation Flow' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }

  describe 'Login -> Profile -> Logout flow' do
    it 'allows user to login, visit profile, and logout successfully' do
      # Clear any 2FA setup to allow direct login
      account = user.person.account
      AccountOtpKey.where(id: account.id).delete_all

      # Step 1: Login
      visit '/login'
      expect(page).to have_content('Welcome back')

      fill_in 'Email address', with: user.email_address
      fill_in 'Password', with: 'password'
      click_button 'Sign In to Dashboard'

      # Verify login successful
      expect(page).to have_current_path('/dashboard')
      expect(page).to have_content(user.name)

      # Step 2: Visit Profile via direct navigation (dropdown may be obscured by flash)
      visit '/profile'

      expect(page).to have_current_path('/profile')
      expect(page).to have_content('My Profile')
      expect(page).to have_content(user.name)
      expect(page).to have_content('Personal Information')
      expect(page).to have_content('Account Security')

      # Step 3: Logout - wait for warning flash to auto-dismiss before clicking nav
      expect(page).to have_no_css('[data-controller="flash"]', wait: 10)
      click_button 'Sign Out'

      # Verify logout successful
      using_wait_time(5) do
        expect(page).to have_current_path('/login')
        expect(page).to have_button('Sign In to Dashboard')
      end
    end
  end

  describe 'Direct navigation when not logged in' do
    it 'redirects to login when accessing profile' do
      visit '/profile'
      expect(page).to have_current_path('/login')
    end

    it 'redirects to login when accessing dashboard' do
      visit '/dashboard'
      expect(page).to have_current_path('/login')
    end
  end
end
