# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication dose options editor' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  before do
    driven_by(:playwright)
    login_as(users(:admin))
  end

  it 'applies frequency templates and supports remove/undo for persisted rows' do
    visit edit_medication_path(medications(:calpol))

    within first("[data-dosage-options-target='option']") do
      expect(page).to have_field('medication_dosage_records_attributes_0_display_unit', with: 'ml', disabled: true)

      click_button 'Every morning'

      expect(page).to have_field('medication[dosage_records_attributes][0][frequency]', with: 'Every morning')
      expect(page).to have_field('medication[dosage_records_attributes][0][default_max_daily_doses]', with: '1')
      expect(
        page
      ).to have_field('medication[dosage_records_attributes][0][default_min_hours_between_doses]', with: '24')
      expect(page).to have_select('medication[dosage_records_attributes][0][default_dose_cycle]', selected: 'Daily')

      click_button 'Every 4-6 hours'

      expect(page).to have_field('medication[dosage_records_attributes][0][frequency]', with: 'Every 4-6 hours')
      expect(page).to have_field('medication[dosage_records_attributes][0][default_max_daily_doses]', with: '6')
      expect(page).to have_field('medication[dosage_records_attributes][0][default_min_hours_between_doses]', with: '4')
      expect(page).to have_select('medication[dosage_records_attributes][0][default_dose_cycle]', selected: 'Daily')

      click_button 'Remove dose option'

      expect(page).to have_content('Dose option removed')
      expect(page).to have_button('Undo')

      click_button 'Undo'

      expect(page).to have_button('Remove dose option')
      expect(page).to have_field('medication[dosage_records_attributes][0][frequency]', with: 'Every 4-6 hours')
    end
  end
end
