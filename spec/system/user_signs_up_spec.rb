# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'UserSignsUps' do
  fixtures :accounts, :account_otp_keys, :people, :users

  before do
    driven_by(:rack_test)
  end

  # TODO: This test is for legacy UsersController signup (Phase 5 removal)
  # Will be replaced with Rodauth signup flow - see RODAUTH_SIGNUP_IMPLEMENTATION.md
  it 'allows a user to sign up and prompts for email verification' do
    pending 'This test is for legacy UsersController signup (Phase 5 removal)'

    visit sign_up_path

    fill_in 'Name', with: 'New User'
    fill_in 'Date of birth', with: '1995-05-15'
    fill_in 'Email address', with: 'newuser@example.com'
    fill_in 'Password', with: 'password'
    fill_in 'Password confirmation', with: 'password'

    click_button 'Sign Up'

    aggregate_failures 'user sign up' do
      expect(page).to have_css('#flash', text: 'Check your email to verify your account before signing in.')

      new_user = User.last
      expect(new_user.email_address).to eq('newuser@example.com')
      expect(new_user.role).to eq('parent')
      expect(new_user.person.person_type).to eq('adult')
    end
  end
end
