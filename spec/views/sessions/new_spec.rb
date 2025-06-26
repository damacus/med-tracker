# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Sessions::New view', type: :system do
  # Load fixtures at the example group level, not inside examples
  fixtures :users

  before do
    driven_by(:selenium_headless)
  end

  it 'renders the login form with all necessary fields' do
    visit new_session_path

    aggregate_failures 'login form' do
      expect(page).to have_field('email_address')
      expect(page).to have_field('password')
      expect(page).to have_button('Sign in')
      expect(page).to have_link('Forgot password?')
    end
  end

  it 'shows alert messages when login fails' do
    visit new_session_path
    fill_in 'email_address', with: 'wrong@example.com'
    fill_in 'password', with: 'wrongpassword'
    click_button 'Sign in'

    within 'Login' do
      aggregate_failures 'login form' do
        expect(page).to have_content('Try another email address or password')
      end
    end
  end

  it 'shows notice messages upon successful actions' do
    user = users(:john)

    visit login_path
    fill_in 'email_address', with: user.email_address
    fill_in 'password', with: 'password'
    click_button 'Sign in'

    within 'Login' do
      aggregate_failures 'login form' do
        expect(page).to have_content('Signed in successfully')
      end
    end
  end
end
