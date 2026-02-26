# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationNewLayout' do
  fixtures :accounts, :people, :users, :locations

  before do
    driven_by(:rack_test)
  end

  it 'allows creating a medication with the redesigned form' do
    sign_in(users(:john))

    visit new_medication_path

    expect(page).to have_content('Add a New Medication')
    expect(page).to have_content('Capture inventory details and dosage information.')

    within('[data-testid="medication-form"]') do
      aggregate_failures 'form fields' do
        expect(page).to have_field('Name')
        expect(page).to have_field('Description')
        expect(page).to have_field('Standard Dosage')
        expect(page).to have_select('Unit')
        expect(page).to have_select('Unit', with_options: ['sachet'])
        expect(page).to have_field('Remaining Supply')
        expect(page).to have_field('Reorder Threshold')
        expect(page).to have_field('Warnings')
      end

      select 'Home', from: 'Location'
      fill_in 'Name', with: 'Ibuprofen'
      fill_in 'Description', with: 'Pain relief'
      fill_in 'Standard Dosage', with: 200
      select 'mg', from: 'Unit'
      fill_in 'Remaining Supply', with: 40
      fill_in 'Reorder Threshold', with: 10
      fill_in 'Warnings', with: 'Take with food'

      click_button 'Save Medication'
    end

    aggregate_failures 'persistence' do
      expect(page).to have_current_path(medication_path(Medication.last))
      expect(page).to have_content('Medication was successfully created.')
      expect(Medication.last.attributes).to include(
        'name' => 'Ibuprofen',
        'description' => 'Pain relief',
        'dosage_amount' => 200.0,
        'dosage_unit' => 'mg',
        'current_supply' => 40,
        'warnings' => 'Take with food'
      )
    end
  end
end
