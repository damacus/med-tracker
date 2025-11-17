# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Sessions' do
  include Rails.application.routes.url_helpers

  describe 'login page' do
    before do
      visit '/login'
    end

    it 'displays the login form' do
      expect(page).to have_field('Email address')
      expect(page).to have_field('Password')
      expect(page).to have_button('Login')
      expect(page).to have_link('Forgot password?')
    end

    it 'maintains the email value after a failed attempt' do
      fill_in 'Email address', with: 'test@example.com'
      fill_in 'Password', with: 'wrongpassword'
      click_button 'Login'

      expect(page).to have_field('Email address', with: 'test@example.com')
    end

    it 'displays error message inside the login card' do
      fill_in 'Email address', with: 'test@example.com'
      fill_in 'Password', with: 'wrongpassword'
      click_button 'Login'

      expect(page).to have_css("#login-flash [role='alert']",
                               text: 'There was an error logging in')
    end
  end
end
