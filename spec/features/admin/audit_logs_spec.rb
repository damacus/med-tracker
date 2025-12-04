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

  # AUDIT-013: Complete audit trail review workflow
  describe 'complete audit trail review workflow (AUDIT-013)' do
    before { sign_in(admin) }

    it 'displays list of audit entries with required columns' do
      # Create audit entries
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end

      visit admin_audit_logs_path

      # Verify list is displayed
      expect(page).to have_css('[data-testid="admin-audit-logs"]')

      # Verify columns: timestamp, record type, event, user
      within('thead') do
        expect(page).to have_content('Timestamp')
        expect(page).to have_content('Record Type')
        expect(page).to have_content('Event')
        expect(page).to have_content('User')
      end
    end

    it 'shows detailed view with previous and new state' do
      # Create an update audit entry
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end

      visit admin_audit_logs_path
      first('a', text: 'View').click

      # Verify detailed view
      expect(page).to have_content('Audit Log Details')
      expect(page).to have_content('Event Information')

      # Verify previous state is displayed
      expect(page).to have_content('Previous State')

      # Verify new state is displayed
      expect(page).to have_content('New State')
    end

    it 'allows navigation back to list' do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end

      visit admin_audit_logs_path
      first('a', text: 'View').click

      expect(page).to have_link('Back to Audit Logs')
      click_link 'Back to Audit Logs'

      expect(page).to have_current_path(admin_audit_logs_path)
    end

    it 'filters by record type and shows only matching entries' do
      # Create different types of audit entries
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
        people(:john).update!(name: 'John Updated')
      end

      visit admin_audit_logs_path

      # Filter by User
      select 'User', from: 'item_type'

      # Verify only User entries shown
      within('tbody') do
        expect(page).to have_content('User')
        expect(page).to have_no_content('Person')
      end
    end

    it 'filters by event type and shows only matching entries' do
      # Create different event types
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end

      visit admin_audit_logs_path

      # Filter by update event
      select 'Update', from: 'event'

      # Verify only update events shown
      within('tbody') do
        expect(page).to have_content('Update')
      end
    end

    it 'clears all filters and shows all entries again' do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end

      visit admin_audit_logs_path(item_type: 'User')

      # Verify filter is active
      expect(page).to have_link('Clear')

      # Clear filters
      click_link 'Clear'

      # Verify all entries shown
      expect(page).to have_current_path(admin_audit_logs_path)
    end
  end

  # AUDIT-014: Audit trail for medication take lifecycle
  describe 'medication take lifecycle audit (AUDIT-014)' do
    let(:carer) { users(:bob) }
    let(:prescription) { prescriptions(:john_paracetamol) }

    before { sign_in(admin) }

    it 'logs medication take creation with all required fields' do
      # Create a medication take as carer
      PaperTrail.request.whodunnit = carer.id
      PaperTrail.request.controller_info = { ip: '192.168.1.100' }
      PaperTrail.request(enabled: true) do
        MedicationTake.create!(
          prescription: prescription,
          taken_at: Time.current
        )
      end

      visit admin_audit_logs_path

      # Filter by MedicationTake
      select 'Medication Take', from: 'item_type'

      # Verify create event is logged
      within('tbody') do
        expect(page).to have_content('Medication Take')
        expect(page).to have_content('Create')
      end

      # View details
      first('a', text: 'View').click

      # Verify whodunnit shows carer user
      expect(page).to have_content(carer.name)

      # Verify new state contains prescription_id
      expect(page).to have_content('New State')
      expect(page).to have_content('prescription_id')
    end

    it 'records IP address for medication takes' do
      # Create an actual MedicationTake with IP tracking enabled
      # This tests the end-to-end audit trail functionality
      PaperTrail.request.whodunnit = carer.id
      PaperTrail.request.controller_info = { ip: '192.168.1.100' }
      PaperTrail.request(enabled: true) do
        MedicationTake.create!(
          prescription: prescription,
          taken_at: Time.current
        )
      end

      visit admin_audit_logs_path(item_type: 'MedicationTake')

      # Verify IP address is displayed
      within('tbody') do
        expect(page).to have_content('192.168.1.100')
      end
    end
  end

  describe 'pagination' do
    before { sign_in(admin) }

    it 'shows pagination controls when there are many entries' do
      # Create more than 50 audit entries to trigger pagination.
      # NOTE: We create PaperTrail::Version records directly here for performance.
      # Creating 55 actual model changes would be slow and unnecessary since we're
      # testing pagination UI, not the audit trail generation mechanism itself.
      # The audit trail mechanism is tested in other specs.
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        55.times do |i|
          PaperTrail::Version.create!(
            item_type: 'User',
            item_id: i + 1000,
            event: 'update',
            whodunnit: admin.id.to_s,
            created_at: Time.current
          )
        end
      end

      visit admin_audit_logs_path

      # Verify pagination controls are displayed
      expect(page).to have_css('nav[aria-label="Pagination"]')
      # The "Showing X to Y of Z results" text is hidden on small screens (hidden sm:block)
      # so we check for it with visible: :all, or verify the Next button is present
      expect(page).to have_link('Next')
    end

    it 'navigates between pages' do
      # Create more than 50 audit entries.
      # NOTE: Direct Version creation is intentional for performance - see comment above.
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        55.times do |i|
          PaperTrail::Version.create!(
            item_type: 'User',
            item_id: i + 1000,
            event: 'update',
            whodunnit: admin.id.to_s,
            created_at: Time.current
          )
        end
      end

      visit admin_audit_logs_path

      # Click next page
      click_link 'Next'

      expect(page).to have_current_path(/page=2/)
    end
  end
end
