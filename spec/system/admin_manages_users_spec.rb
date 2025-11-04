# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AdminManagesUsers' do
  fixtures :users

  # Use the admin fixture instead of creating a duplicate user
  let(:admin) { users(:admin) }
  # Use a unique email for the carer user to avoid conflicts
  let!(:carer) do
    person = Person.create!(name: 'Carer User', date_of_birth: '1990-01-01')
    User.create!(person: person, email_address: 'test_carer@example.com',
                 password: 'password', password_confirmation: 'password', role: :carer)
  end

  before do
    driven_by(:rack_test)
  end

  context 'when user is logged in as an admin' do
    it 'allows admin to see the list of users' do
      sign_in_as(admin)

      visit admin_users_path

      within '[data-testid="admin-users"]' do
        expect(page).to have_content('User Management')
        expect(page).to have_content(admin.email_address)
        expect(page).to have_content(carer.email_address)
      end
    end
  end

  context 'when user is logged in as a non-admin' do
    it 'denies access to the user list' do
      sign_in_as(carer)

      visit admin_users_path

      expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')
    end
  end

  def sign_in_as(user, password: 'password')
    visit login_path
    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: password
    click_button 'Sign in'
  end
end
