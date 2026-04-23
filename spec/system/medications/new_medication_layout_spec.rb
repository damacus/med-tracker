# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationNewLayout' do
  fixtures :accounts, :people, :users, :locations, :medications

  before do
    driven_by(:rack_test)
  end

  it 'allows creating a medication with the redesigned form' do
    sign_in(users(:john))

    visit new_medication_path

    expect(page).to have_content('Add a New Medication')

    within('[data-testid="medication-wizard-form"]') do
      aggregate_failures 'form fields' do
        expect(page).to have_field('Name')
        expect(page).to have_field('Description')
        expect(page).to have_field('Dose')
        expect(page).to have_field('medication[dosage_unit]', type: :radio, with: 'mg')
        expect(page).to have_field('medication[dosage_unit]', type: :radio, with: 'sachet')
        expect(page).to have_field('Starting Supply')
        expect(page).to have_field('Reorder Threshold')
        expect(page).to have_field('Warnings')
      end

      choose('medication[location_id]', option: locations(:home).id.to_s)
      fill_in 'Name', with: 'Ibuprofen'
      fill_in 'Description', with: 'Pain relief'
      fill_in 'Dose', with: 200
      choose('medication[dosage_unit]', option: 'mg')
      fill_in 'Starting Supply', with: 40
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
        'current_supply' => 40,
        'warnings' => 'Take with food'
      )
    end
  end

  it 'defaults location to the signed-in user primary location' do
    sign_in(users(:jane))

    visit new_medication_path

    expect(page).to have_field('medication[location_id]', type: :radio, checked: true)
  end
end
