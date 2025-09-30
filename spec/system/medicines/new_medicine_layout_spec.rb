# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicineNewLayout', type: :system do
  fixtures :users

  before do
    driven_by(:rack_test)
  end

  it 'allows creating a medicine with the redesigned form' do
    sign_in(users(:john))

    visit new_medicine_path

    within('header') do
      expect(page).to have_content('Add a new medicine')
      expect(page).to have_content('Capture inventory details and dosage information.')
    end

    within('[data-testid="medicine-form"]') do
      aggregate_failures 'form fields' do
        expect(page).to have_field('Name')
        expect(page).to have_field('Description')
        expect(page).to have_field('Standard dosage')
        expect(page).to have_select('Unit')
        expect(page).to have_field('Current supply')
        expect(page).to have_field('Stock')
        expect(page).to have_field('Warnings')
      end

      fill_in 'Name', with: 'Ibuprofen'
      fill_in 'Description', with: 'Pain relief'
      fill_in 'Standard dosage', with: 200
      select 'mg', from: 'Unit'
      fill_in 'Current supply', with: 40
      fill_in 'Stock', with: 80
      fill_in 'Warnings', with: 'Take with food'

      click_button 'Save medicine'
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
