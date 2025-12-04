# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'User Sessions', :js do
  fixtures :accounts, :account_otp_keys, :people, :users
  let(:user) { users(:jane) }

  describe 'login page' do
    it 'displays the login form with all fields' do
      visit login_path

      within 'main' do
        aggregate_failures 'login form' do
          expect(page).to have_field('email')
          expect(page).to have_field('password')
          expect(page).to have_button('Login')
          expect(page).to have_link('Forgot password?')
        end
      end
    end

    it 'shows error messages for invalid login' do
      visit login_path

      fill_in 'email', with: 'wrong@example.com'
      fill_in 'password', with: 'wrongpass'
      click_button 'Login'

      using_wait_time(3) do
        within '#flash' do
          aggregate_failures 'flash messages' do
            expect(page).to have_content('error logging in')
          end
        end
      end
    end

    it 'allows user to login with valid credentials' do
      login_as(user)

      using_wait_time(3) do
        within '#flash' do
          aggregate_failures 'flash messages' do
            expect(page).to have_content('You have been logged in')
          end
        end
      end
    end
  end

  describe 'logout' do
    it 'allows a logged in user to sign out' do
      login_as(user)

      using_wait_time(5) do
        expect(page).to have_current_path('/dashboard')
        expect(page).to have_content('You have been logged in')
      end

      click_button user.name
      click_link 'Logout'

      using_wait_time(5) do
        expect(page).to have_link('Login', href: '/login')
        expect(page).to have_no_button(user.name)
      end
    end
  end
end
