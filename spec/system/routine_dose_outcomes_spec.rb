require 'rails_helper'

RSpec.describe 'Viewing routine dose outcomes', :browser do
  fixtures :households, :accounts, :people, :users, :locations, :medications, :dosages

  before do
    driven_by(:playwright)
    sign_in(users(:john))
    source = create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c))
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 1, outcome: 'not_taken', reason: 'unwell',
      note: 'Routine decision recorded', resolved_at: Time.current,
      resolved_by_membership: accounts(:john_doe).first_active_household_membership
    )
  end

  [1280, 390].each do |width|
    it "shows a routine decision at #{width}px without another pending dose" do
      page.current_window.resize_to(width, 844)
      visit dashboard_path
      expect(page).to have_css('[data-testid="dashboard-not-taken-outcome"]', text: 'Routine decision recorded')
      expect(page).to have_no_css('[data-testid="dashboard-routine-task"]', text: medications(:vitamin_c).display_name)
    end
  end
end
