# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin Dashboard' do
  fixtures :accounts, :people, :users

  context 'when user is an administrator' do
    it 'displays key system metrics' do
      sign_in(users(:admin))

      visit admin_root_path

      within '[data-testid="admin-dashboard"]' do
        expect(page).to have_content('Admin Dashboard')

        # Check for metric cards
        expect(page).to have_css('[data-testid="metric-total-users"]')
        expect(page).to have_css('[data-testid="metric-total-people"]')
        expect(page).to have_css('[data-testid="metric-active-schedules"]')
        expect(page).to have_css('[data-testid="metric-patients-without-carers"]')
      end
    end

    it 'displays correct user counts by role' do
      sign_in(users(:admin))

      visit admin_root_path

      within '[data-testid="metric-total-users"]' do
        expect(page).to have_content('Total Users')
        expect(page).to have_css('[data-metric-value]')
      end
    end

    it 'provides navigation links to management pages' do
      sign_in(users(:admin))

      visit admin_root_path

      expect(page).to have_link('Manage Users', href: admin_users_path)
      expect(page).to have_link('Manage People', href: people_path)
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

      expect(page).to have_current_path(login_path)
    end
  end
end
