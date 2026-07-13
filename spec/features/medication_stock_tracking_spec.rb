# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Stock Tracking', type: :system do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  let(:admin) { users(:admin) }
  let(:person) { people(:john) }
  let(:medication) { medications(:paracetamol) }
  let(:schedule) { schedules(:john_paracetamol) }

  before do
    # Ensure medication has known stock level using current_supply
    medication.update!(current_supply: 10, reorder_threshold: 5)

    # Clear any fixture medication_takes to avoid cooldown interference
    MedicationTake.where(schedule: schedule).delete_all

    # Login as admin using helper that clears 2FA
    login_as(admin)
  end

  it 'displays current stock on the person profile' do
    visit person_path(person)

    within "##{tenant_dom_id(schedule)}" do
      # Badge doesn't show when adequately stocked (only for low/out of stock)
      expect(page).to have_no_text('In Stock')
      expect(page).to have_text('10')
    end
  end

  it 'shows low stock badge when stock reaches threshold' do
    medication.update!(current_supply: 5)
    visit person_path(person)

    within "##{tenant_dom_id(schedule)}" do
      expect(page).to have_text('Low Stock')
      expect(page).to have_text('5')
    end
  end

  it 'shows out of stock badge when stock is zero' do
    medication.update!(current_supply: 0)
    visit person_path(person)

    within "##{tenant_dom_id(schedule)}" do
      expect(page).to have_text('Out of Stock')
      expect(page).to have_text('0')
    end
  end

  it 'deducts stock when taking a dose via the dashboard', :browser do
    visit dashboard_path(dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID)

    # Ensure the task is available on the dashboard
    expect(page).to have_text('Paracetamol')

    as_needed_card_for(schedule).find('summary').click
    find("[data-testid='take-dose-schedule_#{schedule.id}']").click
    confirm_record_dose(take_medication_person_schedule_path(person, schedule), I18n.t('person_medications.card.give'))

    expect(page).to have_text(/taken successfully/)

    # Verify stock reduction on the person profile page
    visit person_path(person)
    within "##{tenant_dom_id(schedule)}" do
      expect(page).to have_text('9 left')
    end
  end

  def confirm_record_dose(path, label)
    within("form[action='#{path}']") do
      click_button label
    end
  end

  def as_needed_card_for(schedule)
    all('details[data-testid="dashboard-as-needed-person"]', visible: :all).find do |details|
      details.has_css?("##{tenant_dom_id(schedule, :timeline)}", visible: :all)
    end
  end
end
