# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Sessions::New view', type: :system do
  # Load fixtures at the example group level, not inside examples
  fixtures :accounts, :people, :users

  before do
    driven_by(:playwright)
  end

  it 'renders the login form with all necessary fields' do
    visit new_session_path

    aggregate_failures 'login form' do
      expect(page).to have_field('email_address')
      expect(page).to have_field('password')
      expect(page).to have_button('Login')
      expect(page).to have_link('Forgot password?')
    end
  end

  it 'applies the styled layout' do
    visit new_session_path

    aggregate_failures 'styled layout' do
      expect(page).to have_css('h1', text: 'MedTracker')
      expect(page).to have_css('p', text: 'Sign in to manage your medication plan')
      expect(page).to have_css('form.space-y-6')
      expect(page).to have_button('Login', type: 'submit')
      expect(page).to have_css('div[data-controller="ruby-ui--form-field"]', count: 2)
      expect(page).to have_css('input[data-ruby-ui--form-field-target="input"]', count: 2)
      expect(page).to have_css('a.text-primary', text: 'Forgot password?')
      expect(page).to have_css('div.bg-gradient-to-br')
    end
  end

  it 'shows alert messages when login fails' do
    visit new_session_path
    fill_in 'Email address', with: 'wrong@example.com'
    fill_in 'Password', with: 'wrongpassword'
    click_button 'Login'

    aggregate_failures 'login form' do
      expect(page).to have_content('Try another email address or password')
    end
  end

  it 'shows notice messages upon successful actions' do
    user = users(:john)

    visit login_path
    fill_in 'email', with: user.email_address
    fill_in 'password', with: 'password'
    click_button 'Login'

    expect(page).to have_content('You have been logged in')
  end
end
