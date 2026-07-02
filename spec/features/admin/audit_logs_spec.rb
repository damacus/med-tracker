# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin Audit Logs', type: :system do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }

  after do
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
  end

  describe 'access control' do
    it 'allows administrators to view audit logs' do
      sign_in(admin)
      visit admin_audit_logs_path

      expect(page).to have_text('Audit Trail')
      expect(page).to have_text('Complete history of all changes')
    end

    it 'denies access to non-administrators' do
      sign_in(regular_user)
      visit admin_audit_logs_path

      expect(page).to have_text('not authorized')
    end

    it 'redirects unauthenticated users to login' do
      visit admin_audit_logs_path

      expect(page).to have_current_path(login_path)
    end
  end

  describe 'navigation' do
    before { sign_in_with_audit_context(admin) }

    it 'shows audit trail link on admin dashboard' do
      visit admin_root_path

      expect(page).to have_link('Audit Trail')
      click_link 'Audit Trail'

      expect(page).to have_current_path(admin_audit_logs_path)
    end
  end

  describe 'audit log list' do
    before { sign_in_with_audit_context(admin) }

    it 'displays the audit log page with headers' do
      visit admin_audit_logs_path

      expect(page).to have_text('Audit Trail')
      expect(page).to have_text('Complete history of all changes')
      expect(page).to have_button('All Types')
      expect(page).to have_button('All Events')
    end

    it 'includes Medication in record type filter options' do
      visit admin_audit_logs_path

      click_button 'All Types'

      expect(page).to have_css('label[role="option"]', text: 'Medication')
    end

    it 'shows filter form with searchable comboboxes' do
      visit admin_audit_logs_path

      expect(page).to have_css('[role="combobox"]', count: 2)
      expect(page).to have_css('input[type="search"][role="searchbox"]', visible: :all, count: 2)
    end
  end

  describe 'filtering' do
    before { sign_in_with_audit_context(admin) }

    it 'has filter form with Stimulus controller' do
      visit admin_audit_logs_path

      # Verify form has the filter-form Stimulus controller
      expect(page).to have_css('form[data-controller="filter-form"]')
    end

    it 'has dropdowns with auto-submit actions' do
      visit admin_audit_logs_path

      # Verify filter radios have the change action wired to Stimulus
      expect(page).to have_css(
        'input[type="radio"][name="item_type"][data-action*="filter-form#submit"]',
        visible: :all
      )
      expect(page).to have_css('input[type="radio"][name="event"][data-action*="filter-form#submit"]', visible: :all)
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
    before { sign_in_with_audit_context(admin) }

    it 'shows detail page when clicking View link' do
      # Create a test audit entry
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
      end

      visit admin_audit_logs_path

      # Click on first View link
      first('a', text: 'View').click

      expect(page).to have_text('Audit Log Details')
      expect(page).to have_text('Event Information')
      expect(page).to have_link('Back to Audit Logs')
    end
  end

  # AUDIT-013: Complete audit trail review workflow
  describe 'complete audit trail review workflow (AUDIT-013)' do
    before { sign_in_with_audit_context(admin) }

    it 'displays list of audit entries with required columns' do
      # Create audit entries
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
      end

      visit admin_audit_logs_path

      # Verify list is displayed
      expect(page).to have_css('[data-testid="admin-audit-logs"]')

      # Verify columns: time, record type, event, user
      within('thead') do
        expect(page).to have_text('Time')
        expect(page).to have_text('Record Type')
        expect(page).to have_text('Event')
        expect(page).to have_text('User')
      end
    end

    it 'shows detailed view with previous and new state' do
      # Create an update audit entry
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
      end

      visit admin_audit_logs_path
      first('a', text: 'View').click

      # Verify detailed view
      expect(page).to have_text('Audit Log Details')
      expect(page).to have_text('Event Information')

      # Verify previous state is displayed
      expect(page).to have_text('Previous State')

      # Verify new state is displayed
      expect(page).to have_text('New State')
    end

    it 'allows navigation back to list' do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
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
        users(:jane).update!(active: false)
        people(:john).update!(name: 'John Updated')
      end

      visit admin_audit_logs_path

      # Filter by User
      click_button 'All Types'
      find('label[role="option"]', text: 'User').click

      # Verify only User entries shown
      within('tbody') do
        expect(page).to have_text('User')
        expect(page).to have_no_text('Person')
      end
    end

    it 'filters by event type and shows only matching entries' do
      # Create different event types
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
      end

      visit admin_audit_logs_path

      # Filter by update event
      click_button 'All Events'
      find('label[role="option"]', text: 'Update').click

      # Verify only update events shown
      within('tbody') do
        expect(page).to have_text('Update')
      end
    end

    it 'clears all filters and shows all entries again' do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
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
    let(:schedule) { schedules(:john_paracetamol) }

    before { sign_in_with_audit_context(admin) }

    it 'logs medication take creation with all required fields' do
      # Create a medication take as carer
      PaperTrail.request.whodunnit = carer.id
      set_audit_context(carer, ip: '192.168.1.100')
      PaperTrail.request(enabled: true) do
        MedicationTake.create!(
          schedule: schedule,
          taken_at: Time.current,
          dose_amount: 10.0
        )
      end

      # Visit with filter applied directly to avoid Turbo timing issues
      visit admin_audit_logs_path(item_type: 'MedicationTake')

      # Verify create event is logged
      within('tbody') do
        expect(page).to have_text('Medication Take')
        expect(page).to have_text('Create')
      end

      # View details
      click_link 'View', match: :first

      # Verify whodunnit shows carer user
      expect(page).to have_text(carer.name)

      # Verify new state contains schedule_id
      expect(page).to have_text('New State')
      expect(page).to have_text('schedule_id')
    end

    it 'records IP address for medication takes' do
      # Create an actual MedicationTake with IP tracking enabled
      # This tests the end-to-end audit trail functionality
      PaperTrail.request.whodunnit = carer.id
      set_audit_context(carer, ip: '192.168.1.100')
      PaperTrail.request(enabled: true) do
        MedicationTake.create!(
          schedule: schedule,
          taken_at: Time.current,
          dose_amount: 10.0
        )
      end

      visit admin_audit_logs_path(item_type: 'MedicationTake')

      # Verify IP address is displayed
      within('tbody') do
        expect(page).to have_text('192.168.1.100')
      end
    end
  end

  describe 'medication restock audit trail' do
    let(:medication) { medications(:paracetamol) }

    before { sign_in_with_audit_context(admin) }

    it 'shows restock entries for medications in the audit log' do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        medication.paper_trail_event = 'restock'
        medication.restock!(quantity: 7)
      end

      visit admin_audit_logs_path(item_type: 'Medication')

      within('tbody') do
        expect(page).to have_text('Medication')
        expect(page).to have_text('Restock')
      end

      click_link 'View', match: :first

      expect(page).to have_text('New State')
      expect(page).to have_text('current_supply')
      expect(page).to have_text('stock')
    end
  end

  describe 'pagination' do
    before { sign_in_with_audit_context(admin) }

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
            household_id: current_audit_household.id,
            actor_membership_id: current_audit_membership&.id,
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
            household_id: current_audit_household.id,
            actor_membership_id: current_audit_membership&.id,
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

  def sign_in_with_audit_context(user)
    sign_in(user)
    set_audit_context(user)
  end

  def set_audit_context(user, ip: nil)
    PaperTrail.request.controller_info = {
      household_id: current_audit_household(user).id,
      actor_membership_id: current_audit_membership(user)&.id,
      ip: ip
    }.compact
  end

  def current_audit_household(user = admin)
    ensure_api_household_for(user)
  end

  def current_audit_membership(user = admin)
    account = Account.find_by(email: user.email_address)
    current_audit_household(user).household_memberships.find_by(account: account)
  end
end
