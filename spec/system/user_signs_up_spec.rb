# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'UserSignsUps', type: :system do
  before do
    driven_by(:rack_test)
  end

  it 'allows a user to sign up' do
    visit '/sign_up'

    fill_in 'Name', with: 'New User'
    fill_in 'Date of birth', with: '1995-05-15'
    fill_in 'Email address', with: 'newuser@example.com'
    fill_in 'Password', with: 'password'
    fill_in 'Password confirmation', with: 'password'

    click_button 'Sign Up'

    within 'UserSignsUps' do
      aggregate_failures 'user sign up' do
        expect(page).to have_content('Welcome! You have signed up successfully.')
        expect(User.count).to eq(7) # New user + users.yml fixture
        expect(User.last.email_address).to eq('newuser@example.com')
      end
    end
  end
end
