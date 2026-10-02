require 'rails_helper'

RSpec.describe 'Dashboard dose outcomes', :browser do
  fixtures :households, :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  before do
    driven_by(:playwright)
    sign_in(users(:john))
    source = schedules(:john_movicol)
    actor = accounts(:john_doe).household_memberships.find_by!(household: source.household)
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 1, outcome: 'not_taken', reason: 'unwell', note: 'Feeling unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  it 'shows the recorded outcome on desktop and mobile without an outstanding task' do
    [1280, 390].each do |width|
      page.current_window.resize_to(width, 844)
      visit dashboard_path

      outcome = find('[data-testid="dashboard-not-taken-outcome"]')
      expect(outcome).to have_text('Not taken')
      expect(outcome).to have_text('Unwell')
      expect(outcome).to have_text('Feeling unwell')
      expect(page).to have_no_css('[data-testid="dashboard-routine-task"]',
                                  text: schedules(:john_movicol).medication.display_name)
    end
  end
end
