# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard' do
  fixtures :accounts, :users, :locations, :medications, :dosages, :schedules, :people,
           :carer_relationships, :person_medications, :medication_takes

  it 'loads the dashboard for a signed-in user and shows family-wide doses' do
    travel_to(Time.current.beginning_of_day + 9.hours) do
      sign_in(users(:jane))

      visit dashboard_path

      expect(page).to have_content('Good morning')
      expect(page).to have_content("Today's Schedule")

      # Jane's medication
      expect(page).to have_content('Ibuprofen')
      expect(page).to have_content('Jane Doe')

      # Child's medication
      expect(page).to have_content('Child Patient')
    end
  end

  it 'allows taking a dose directly from the dashboard' do
    sign_in(users(:jane))
    visit dashboard_path

    # Wait for the dashboard to load and show schedules
    expect(page).to have_content("Today's Schedule")

    # Find the first take-dose button
    button = find('[data-testid^="take-dose-"]')

    expect do
      button.click
      # Wait for the success message
      expect(page).to have_content('Medication taken successfully.', wait: 10)
    end.to change(MedicationTake, :count).by(1)
  end
end
