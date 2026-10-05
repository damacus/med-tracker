require 'rails_helper'

RSpec.describe 'Correcting a routine dose decision', :browser do
  fixtures :households, :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  let(:source) do
    create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                         dose_cycle: :monthly, max_daily_doses: 1, created_at: 2.months.ago)
  end

  before do
    driven_by(:playwright)
    sign_in(users(:john))
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current.beginning_of_month, position: 1, outcome: 'not_taken',
      reason: 'unwell', note: 'Feeling unwell',
      resolved_at: Time.current, resolved_by_membership: accounts(:john_doe).first_active_household_membership
    )
  end

  [1280, 390].each do |width|
    it "reopens a decision at #{width}px and returns it to the dashboard" do
      page.current_window.resize_to(width, 844)
      visit dashboard_path
      click_link 'Correct decision', exact: true
      expect(page).to have_css('h1', text: 'Correct dose decision')
      expect(page).to have_text('Feeling unwell')
      expect(page).to have_text(Date.current.end_of_month.iso8601)
      click_button 'Reopen dose'
      expect(page).to have_css('[data-testid="dashboard-routine-task"]', text: source.medication.display_name)
      expect(page).to have_no_css('[data-testid="dashboard-not-taken-outcome"]', text: source.medication.display_name)
    end
  end

  it 'records the actual dose from the correction form on mobile' do
    page.current_window.resize_to(390, 844)
    visit dashboard_path
    click_link 'Correct decision', exact: true
    click_button 'Record as taken'
    expect(page).to have_text('Dose decision corrected.')
    expect(source.medication_dose_occurrences.sole).to be_taken
  end
end
