# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard' do
  fixtures :accounts, :users, :locations, :medications, :dosages, :schedules, :people,
           :carer_relationships, :person_medications, :medication_takes

  it 'loads the dashboard and allows taking a dose from the timeline' do
    travel_to(Time.current.beginning_of_day + 9.hours) do
      sign_in(users(:jane))
      visit dashboard_path

      expect(page).to have_content('Good morning')
      expect(page).to have_content("Today's Schedule")
      expect(page).to have_content('Ibuprofen')
      expect(page).to have_content('Jane Doe')
      expect(page).to have_content('Child Patient')

      button = first('[data-testid^="take-dose-"]')

      expect do
        button.click
        expect(page).to have_content('Medication taken successfully.', wait: 10)
      end.to change(MedicationTake, :count).by(1)
    end
  end
end
