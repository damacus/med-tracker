# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AdminManagesUsers', type: :system do
  fixtures :users

  # Use the admin fixture instead of creating a duplicate user
  let(:admin) { users(:admin) }
  # Use a unique email for the carer user to avoid conflicts
  let!(:carer) do
    User.create!(name: 'Carer User', date_of_birth: '1990-01-01', email_address: 'test_carer@example.com',
                 password: 'password', password_confirmation: 'password', role: :carer)
  end

  before do
    driven_by(:rack_test)
  end

  context 'when user is logged in as an admin' do
    it 'allows admin to see the list of users', pending: 'Admin management not yet implemented' do
      # Sign in as admin
      visit login_path
      fill_in 'Email address', with: admin.email_address
      fill_in 'Password', with: 'password'
      click_button 'Sign in'

      # Visit admin users page
      visit admin_users_path

      within 'Users' do
        aggregate_failures 'user list' do
          expect(page).to have_content(admin.email_address)
          expect(page).to have_content(carer.email_address)
          expect(page).to have_content('User Management')
        end
      end
    end
  end

  context 'when user is logged in as a non-admin' do
    it 'denies access to the user list', pending: 'Admin management not yet implemented' do
      # Sign in as carer
      visit login_path
      fill_in 'Email address', with: carer.email_address
      fill_in 'Password', with: 'password'
      click_button 'Sign in'

      # Visit admin users page
      visit admin_users_path

      within 'Users' do
        aggregate_failures 'user list' do
          expect(page).to have_content('You are not authorized to perform this action.')
          expect(page).to have_content(carer.email_address)
          expect(page).to have_content('User Management')
        end
      end
    end
  end
end
