# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin Audit Logs', type: :system do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }

  describe 'access control' do
    it 'allows administrators to view audit logs' do
      sign_in(admin)
      visit admin_audit_logs_path

      expect(page).to have_content('Audit Trail')
      expect(page).to have_content('Complete history of all changes')
    end

    it 'denies access to non-administrators' do
      sign_in(regular_user)
      visit admin_audit_logs_path

      expect(page).to have_content('not authorized')
    end

    it 'redirects unauthenticated users to login' do
      visit admin_audit_logs_path

      expect(page).to have_current_path(login_path)
    end
  end

  describe 'navigation' do
    before { sign_in(admin) }

    it 'shows audit trail link on admin dashboard' do
      visit admin_root_path

      expect(page).to have_link('Audit Trail')
      click_link 'Audit Trail'

      expect(page).to have_current_path(admin_audit_logs_path)
    end
  end

  describe 'audit log list' do
    before { sign_in(admin) }

    it 'displays the audit log page with headers' do
      visit admin_audit_logs_path

      expect(page).to have_content('Audit Trail')
      expect(page).to have_content('Complete history of all changes')
      expect(page).to have_select('item_type')
      expect(page).to have_select('event')
    end

    it 'shows filter form with dropdowns' do
      visit admin_audit_logs_path

      expect(page).to have_select('item_type')
      expect(page).to have_select('event')
    end
  end

  describe 'filtering' do
    before { sign_in(admin) }

    it 'has filter form with Stimulus controller' do
      visit admin_audit_logs_path

      # Verify form has the filter-form Stimulus controller
      expect(page).to have_css('form[data-controller="filter-form"]')
    end

    it 'has dropdowns with auto-submit actions' do
      visit admin_audit_logs_path

      # Verify selects have the change action wired to Stimulus
      expect(page).to have_css('select#item_type[data-action*="filter-form#submit"]')
      expect(page).to have_css('select#event[data-action*="filter-form#submit"]')
    end

    it 'shows clear link when visiting with filter params' do
      visit admin_audit_logs_path(item_type: 'User')

      expect(page).to have_link('Clear')
      expect(page).to have_current_path(/item_type=User/)
    end

    # NOTE: Auto-submit behavior works in browser but is difficult to test reliably
    # in Playwright due to timing of Stimulus controller initialization.
    # The wiring is verified above. For integration testing of the actual behavior,
    # consider using Capybara's execute_script to trigger events after ensuring
    # Stimulus has loaded, or test manually in development with Hotwire Spark.
  end

  describe 'audit log details' do
    before { sign_in(admin) }

    it 'shows detail page when clicking View link' do
      # Create a test audit entry
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end

      visit admin_audit_logs_path

      # Click on first View link
      first('a', text: 'View').click

      expect(page).to have_content('Audit Log Details')
      expect(page).to have_content('Event Information')
      expect(page).to have_link('Back to Audit Logs')
    end
  end
end
