# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Sessions' do
  describe 'login page' do
    before do
      visit login_path
    end

    it 'displays the login form' do
      expect(page).to have_field('Email address')
      expect(page).to have_field('Password')
      expect(page).to have_button('Sign in')
      expect(page).to have_link('Forgot password?')
    end

    it 'maintains the email value after a failed attempt' do
      fill_in 'Email address', with: 'test@example.com'
      fill_in 'Password', with: 'wrongpassword'
      click_button 'Sign in'

      expect(page).to have_field('Email address', with: 'test@example.com')
    end

    it 'displays error message inside the login card' do
      fill_in 'Email address', with: 'test@example.com'
      fill_in 'Password', with: 'wrongpassword'
      click_button 'Sign in'

      expect(page).to have_css("[data-test-id='login-card'] #login-flash [role='alert']",
                               text: 'Try another email address or password.')
    end
  end
end
