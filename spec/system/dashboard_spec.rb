# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard' do
  fixtures :accounts, :users, :locations, :medicines, :dosages, :prescriptions, :people,
           :carer_relationships, :person_medicines, :medication_takes

  it 'loads the dashboard for a signed-in user and shows family-wide doses' do
    travel_to Time.zone.parse('2026-02-25 09:00:00') do
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

    first('[data-testid^="take-dose-"]').click

    expect(page).to have_content('Medicine taken successfully', wait: 10)
  end
end
