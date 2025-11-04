# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin Dashboard' do
  fixtures :users

  context 'when user is an administrator' do
    it 'displays the admin dashboard placeholder' do
      sign_in(users(:admin))

      visit admin_root_path

      within '[data-testid="admin-dashboard"]' do
        aggregate_failures 'admin dashboard content' do
          expect(page).to have_content('Admin Dashboard')
          expect(page).to have_content('Coming Soon')
          expect(page).to have_content('This dashboard is under construction')
        end
      end
    end
  end

  context 'when user is not an administrator' do
    it 'redirects non-admin users' do
      sign_in(users(:carer))

      visit admin_root_path

      expect(page).to have_content('You are not authorized')
      expect(page).to have_current_path(root_path)
    end
  end

  context 'when user is not signed in' do
    it 'redirects to login' do
      visit admin_root_path

      expect(page).to have_current_path(new_session_path)
    end
  end
end
