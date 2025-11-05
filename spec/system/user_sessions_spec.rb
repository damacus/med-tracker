# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'User Sessions', :js do
  fixtures :users
  let(:user) { users(:jane) }

  describe 'login page' do
    it 'displays the login form with all fields' do
      visit login_path

      within 'main' do
        aggregate_failures 'login form' do
          expect(page).to have_field('email_address')
          expect(page).to have_field('password')
          expect(page).to have_button('Sign in')
          expect(page).to have_link('Forgot password?')
        end
      end
    end

    it 'shows error messages for invalid login' do
      visit login_path

      fill_in 'email_address', with: 'wrong@example.com'
      fill_in 'password', with: 'wrongpass'
      click_button 'Sign in'

      using_wait_time(3) do
        within '#flash' do
          aggregate_failures 'flash messages' do
            expect(page).to have_content('Try another email address or password')
          end
        end
      end
    end

    it 'allows user to login with valid credentials' do
      visit login_path

      fill_in 'email_address', with: user.email_address
      fill_in 'password', with: 'password'
      click_button 'Sign in'

      using_wait_time(3) do
        within '#flash' do
          aggregate_failures 'flash messages' do
            expect(page).to have_content('Signed in successfully')
          end
        end
      end
    end
  end

  describe 'logout' do
    it 'allows a logged in user to sign out', pending: 'Turbo DELETE request not completing properly in Playwright' do
      visit login_path
      fill_in 'email_address', with: user.email_address
      fill_in 'password', with: 'password'
      click_button 'Sign in'

      # Wait for the login to complete and redirect
      using_wait_time(5) do
        expect(page).to have_no_current_path(login_path)
        expect(page).to have_content('Signed in successfully')
      end

      # Open the user dropdown menu to access logout button
      click_button user.name

      # Check that we can see the logout button
      expect(page).to have_button('Logout')

      # Click the logout button
      click_button 'Logout'

      # Use Capybara's built-in waiting functionality with a longer timeout
      using_wait_time(5) do
        # Verify we can see the login link in navigation (user is logged out)
        expect(page).to have_link('Login')
        # Verify user is logged out (no Logout button anywhere)
        expect(page).to have_no_button('Logout', visible: :all)
      end
    end
  end
end
