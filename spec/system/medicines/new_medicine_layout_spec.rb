# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicineNewLayout' do
  fixtures :accounts, :account_otp_keys, :people, :users

  before do
    driven_by(:rack_test)
  end

  it 'allows creating a medicine with the redesigned form' do
    sign_in(users(:john))

    visit new_medicine_path

    expect(page).to have_content('Add a New Medicine')
    expect(page).to have_content('Capture inventory details and dosage information.')

    within('[data-testid="medicine-form"]') do
      aggregate_failures 'form fields' do
        expect(page).to have_field('Name')
        expect(page).to have_field('Description')
        expect(page).to have_field('Standard Dosage')
        expect(page).to have_select('Unit')
        expect(page).to have_field('Current Supply')
        expect(page).to have_field('Stock')
        expect(page).to have_field('Reorder Threshold')
        expect(page).to have_field('Warnings')
      end

      fill_in 'Name', with: 'Ibuprofen'
      fill_in 'Description', with: 'Pain relief'
      fill_in 'Standard Dosage', with: 200
      select 'mg', from: 'Unit'
      fill_in 'Current Supply', with: 40
      fill_in 'Stock', with: 80
      fill_in 'Reorder Threshold', with: 10
      fill_in 'Warnings', with: 'Take with food'

      click_button 'Save Medicine'
    end

    aggregate_failures 'persistence' do
      expect(page).to have_current_path(medicine_path(Medicine.last))
      expect(page).to have_content('Medicine was successfully created.')
      expect(Medicine.last.attributes).to include(
        'name' => 'Ibuprofen',
        'description' => 'Pain relief',
        'dosage_amount' => 200.0,
        'dosage_unit' => 'mg',
        'current_supply' => 40,
        'stock' => 80,
        'warnings' => 'Take with food'
      )
    end
  end
end
