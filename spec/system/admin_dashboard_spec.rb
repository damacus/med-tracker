# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin Dashboard' do
  fixtures :accounts, :people, :users

  context 'when user is an administrator' do
    it 'displays key system metrics' do
      sign_in(users(:admin))

      visit admin_root_path

      within '[data-testid="admin-dashboard"]' do
        expect(page).to have_text('Admin Dashboard')
        expect(page).to have_css('[data-testid="dashboard-status"]')
        expect(page).to have_css('[data-testid="attention-queue"]')

        # Check for metric cards
        expect(page).to have_css('[data-testid="metric-total-users"]')
        expect(page).to have_css('[data-testid="metric-total-people"]')
        expect(page).to have_css('[data-testid="metric-active-schedules"]')
        expect(page).to have_css('[data-testid="metric-patients-without-carers"]')
        expect(page).to have_css('[data-testid="metric-pending-invitations"]')
        expect(page).to have_css('[data-testid="metric-recent-audit-events"]')
      end
    end

    it 'displays correct user counts by role' do
      sign_in(users(:admin))

      visit admin_root_path

      within '[data-testid="metric-total-users"]' do
        expect(page).to have_text('Total Users')
        expect(page).to have_css('[data-metric-value]')
      end
    end

    it 'provides navigation links to management pages' do
      sign_in(users(:admin))

      visit admin_root_path

      expect(page).to have_link('Manage Users', href: admin_users_path)
      expect(page).to have_link('Manage People', href: people_path)
      expect(page).to have_link('Audit Trail', href: admin_audit_logs_path)
      expect(page).to have_text('User Access')
      expect(page).to have_text('Operations')
    end
  end

  context 'when user is not an administrator' do
    it 'redirects non-admin users' do
      sign_in(users(:carer))

      visit admin_root_path

      expect(page).to have_text('You are not authorized')
      expect(page).to have_current_path(dashboard_path)
    end
  end

  context 'when user is not signed in' do
    it 'redirects to login' do
      visit admin_root_path

      expect(page).to have_current_path(login_path)
    end
  end
end
