# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Nurse shift medication recording' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :carer_relationships

  let(:nurse) { users(:nurse) }
  let(:john) { people(:john) }
  let(:bob) { people(:bob) }
  let(:john_schedule) { schedules(:john_movicol) }
  let(:bob_schedule) { schedules(:bob_aspirin) }

  before do
    driven_by(:playwright)
    MedicationTake.where(schedule: [john_schedule, bob_schedule]).delete_all
    john_schedule.medication.update!(current_supply: 80)
    bob_schedule.medication.update!(current_supply: 25)
  end

  it 'records doses for multiple patients without crossing patient context' do
    login_as(nurse)
    grant_browser_access(john, access_level: :record)
    grant_browser_access(bob, access_level: :record)

    visit dashboard_path(dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID)
    record_dashboard_dose(john_schedule)
    expect(page).to have_text('Medication taken successfully')

    visit dashboard_path(dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID)
    record_dashboard_dose(bob_schedule)
    expect(page).to have_text('Medication taken successfully')

    john_take = MedicationTake.find_by!(schedule: john_schedule)
    bob_take = MedicationTake.find_by!(schedule: bob_schedule)

    expect(john_take.person).to eq(john)
    expect(john_take.taken_from_medication).to eq(john_schedule.medication)
    expect(bob_take.person).to eq(bob)
    expect(bob_take.taken_from_medication).to eq(bob_schedule.medication)
  end

  def record_dashboard_dose(schedule)
    within("##{tenant_dom_id(schedule, :timeline)}") do
      find("[data-testid='take-dose-schedule_#{schedule.id}']").click
    end

    within("form[action='#{take_medication_person_schedule_path(schedule.person, schedule)}']") do
      click_button 'Give'
    end
  end
end
