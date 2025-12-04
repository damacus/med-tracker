# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Navigation Flow' do
  fixtures :accounts, :account_otp_keys, :people, :users

  let(:user) { users(:damacus) }

  describe 'Login -> Profile -> Logout flow' do
    it 'allows user to login, visit profile, and logout successfully' do
      # Step 1: Login
      visit '/login'
      expect(page).to have_content('Welcome back')

      fill_in 'Email address', with: user.email_address
      fill_in 'Password', with: 'password'
      click_button 'Login'

      # Verify login successful
      expect(page).to have_current_path('/dashboard')
      expect(page).to have_button(user.name)

      # Step 2: Visit Profile
      click_button user.name
      click_link 'Profile'

      expect(page).to have_current_path('/profile')
      expect(page).to have_content('My Profile')
      expect(page).to have_content(user.name)
      expect(page).to have_content('Personal Information')
      expect(page).to have_content('Account Security')

      # Step 3: Logout
      click_button user.name
      click_link 'Logout'

      # Verify logout successful
      using_wait_time(5) do
        expect(page).to have_current_path('/login')
        expect(page).to have_link('Login')
        expect(page).to have_no_button(user.name)
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
