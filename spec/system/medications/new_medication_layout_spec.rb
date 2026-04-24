# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationNewLayout' do
  fixtures :accounts, :people, :users, :locations, :medications

  before do
    driven_by(:playwright)
  end

  it 'allows creating a medication with the redesigned form' do
    sign_in(users(:john))

    visit new_medication_path

    expect(page).to have_content('Add a New Medication')

    within('[data-testid="medication-wizard-form"]') do
      fill_in 'Name', with: 'Ibuprofen'
      fill_in 'Description', with: 'Pain relief'
      click_button 'Continue'

      aggregate_failures 'dose schedule fields' do
        expect(page).to have_content('Who will take this?')
        expect(page).to have_field('Amount')
        expect(page).to have_select('Unit', with_options: %w[mg sachet])
        expect(page).to have_button('Multiple daily')
        expect(page).to have_button('Daily')
        expect(page).to have_button('Weekly')
        expect(page).to have_button('Specific dates')
        expect(page).to have_button('As needed')
        expect(page).to have_button('Tapering')
      end

      fill_in 'Amount', with: 200
      select 'mg', from: 'Unit'
      click_button 'Multiple daily'
      fill_in 'Doses per day', with: 2
      fill_in 'Hours apart', with: 12
      fill_in 'First dose', with: '08:00'
      fill_in 'Second dose', with: '20:00'
      fill_in 'Start date', with: Time.zone.today.to_s
      fill_in 'End date', with: 1.month.from_now.to_date.to_s
      click_button 'Review dose schedule'
      expect(page).to have_content('200 mg, Twice daily')
      expect(page).to have_field('medication_schedule_review_complete', with: 'reviewed', visible: :all)
      click_button 'Continue'

      aggregate_failures 'supply fields' do
        expect(page).to have_field('Starting Supply')
        expect(page).to have_field('Reorder Threshold')
      end
      fill_in 'Starting Supply', with: 40
      fill_in 'Reorder Threshold', with: 10
      click_button 'Continue'
      expect(page).to have_field('Warnings')
      fill_in 'Warnings', with: 'Take with food'

      click_button 'Save Medication'
    end

    aggregate_failures 'persistence' do
      expect(page).to have_content('Ibuprofen created!')
      expect(page).to have_link('Manage dose options')
      expect(page).to have_link('Done')
      expect(Medication.last.attributes).to include(
        'name' => 'Ibuprofen',
        'description' => 'Pain relief',
        'current_supply' => 40,
        'warnings' => 'Take with food'
      )
      expect(Medication.last.dosage_records.last).to have_attributes(
        frequency: 'Twice daily',
        unit: 'mg',
        default_max_daily_doses: 2
      )
    end
  end

  it 'defaults location to the signed-in user primary location' do
    sign_in(users(:jane))

    visit new_medication_path

    checked_location = find("input[name='medication[location_id]'][checked]", visible: :all)

    expect(checked_location.value).to eq(locations(:home).id.to_s)
  end
end
