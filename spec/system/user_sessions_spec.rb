# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'User Sessions', type: :system do
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

      within '#flash' do
        aggregate_failures 'flash messages' do
          expect(page).to have_content('Try another email address or password')
        end
      end
    end

    it 'allows user to login with valid credentials' do
      visit login_path

      fill_in 'email_address', with: user.email_address
      fill_in 'password', with: 'password'
      click_button 'Sign in'

      within '#flash' do
        aggregate_failures 'flash messages' do
          expect(page).to have_content('Signed in successfully')
        end
      end
    end
  end

  describe 'logout' do
    it 'allows a logged in user to sign out' do
      visit login_path
      fill_in 'email_address', with: user.email_address
      fill_in 'password', with: 'password'
      click_button 'Sign in'

      expect(Current.user).not_to be_nil

      click_button 'Sign out'

      within '#flash' do
        aggregate_failures 'flash messages' do
          expect(page).to have_content('Signed out successfully')
        end
      end

      expect(Current.user).to be_nil
    end
  end
end
