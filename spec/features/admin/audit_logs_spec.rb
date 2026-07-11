# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin Audit Logs', :browser, type: :system do
  fixtures :all

  let(:admin) { users(:admin) }

  after do
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
  end

  # AUDIT-013: Complete audit trail review workflow
  it 'reviews, filters, and opens audit entries' do
    sign_in_with_audit_context(admin)
    create_user_and_person_audit_entries

    visit admin_root_path
    click_link 'Audit Trail'

    expect(page).to have_current_path(admin_audit_logs_path)
    expect(page).to have_text('Audit Trail')
    expect(page).to have_css('[data-testid="admin-audit-logs"]')

    within('thead') do
      expect(page).to have_text('Time')
      expect(page).to have_text('Record Type')
      expect(page).to have_text('Event')
      expect(page).to have_text('User')
    end

    click_button 'All Types'
    find('label[role="option"]', text: 'User').click
    expect(page).to have_current_path(/item_type=User/)

    within('tbody') do
      expect(page).to have_text('User')
      expect(page).to have_no_text('Person')
    end

    click_button 'All Events'
    find('label[role="option"]', text: 'Update').click
    expect(page).to have_current_path(/event=update/)

    within('tbody') do
      expect(page).to have_text('Update')
    end

    click_link 'Clear'
    expect(page).to have_current_path(admin_audit_logs_path)

    first('a', text: 'View').click

    expect(page).to have_text('Audit Log Details')
    expect(page).to have_text('Previous State')
    expect(page).to have_text('New State')

    click_link 'Back to Audit Logs'
    expect(page).to have_current_path(admin_audit_logs_path)
  end

  # AUDIT-014: Audit trail for medication take lifecycle
  it 'reviews a medication take audit entry' do
    carer = users(:bob)
    sign_in_with_audit_context(admin)
    create_medication_take_audit_entry(carer)

    visit admin_audit_logs_path(item_type: 'MedicationTake')

    within('tbody') do
      expect(page).to have_text('Medication Take')
      expect(page).to have_text('Create')
      expect(page).to have_text('192.168.1.100')
    end

    click_link 'View', match: :first

    expect(page).to have_text(carer.name)
    expect(page).to have_text('New State')
    expect(page).to have_text('schedule_id')
  end

  it 'reviews a medication restock audit entry' do
    sign_in_with_audit_context(admin)

    PaperTrail.request.whodunnit = admin.id
    PaperTrail.request(enabled: true) do
      medication = medications(:paracetamol)
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

  def create_user_and_person_audit_entries
    PaperTrail.request.whodunnit = admin.id
    PaperTrail.request(enabled: true) do
      users(:jane).update!(active: false)
      people(:john).update!(name: 'John Updated')
    end
  end

  def create_medication_take_audit_entry(carer)
    PaperTrail.request.whodunnit = carer.id
    set_audit_context(carer, ip: '192.168.1.100')
    PaperTrail.request(enabled: true) do
      MedicationTake.create!(
        schedule: schedules(:john_paracetamol),
        taken_at: Time.current,
        dose_amount: 10.0
      )
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
