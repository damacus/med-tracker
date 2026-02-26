# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Refill medication inventory' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before do
    driven_by(:playwright)
    sign_in(admin)
  end

  it 'shows refill actions on inventory pages' do
    visit medications_path
    expect(page).to have_button('Refill Inventory')

    visit location_path(medication.location)
    expect(page).to have_button('Refill Inventory')

    visit medication_path(medication)
    expect(page).to have_button('Refill Inventory')
  end

  it 'refills supply from medication detail with quantity and restock date' do
    visit medication_path(medication)

    click_button 'Refill Inventory'

    expect(page).to have_field('Quantity')
    expect(page).to have_field('Restock date', with: Date.current.to_s)

    fill_in 'Quantity', with: '12'
    fill_in 'Restock date', with: Date.current.to_s
    click_button 'Refill'

    expect(page).to have_content('Inventory refilled successfully.')

    medication.reload
    expect(medication.current_supply).to eq(92)
  end

  it 'shows validation errors when refill quantity is invalid' do
    visit medication_path(medication)

    click_button 'Refill Inventory'
    fill_in 'Quantity', with: '0'
    fill_in 'Restock date', with: Date.current.to_s

    click_button 'Refill'

    expect(page).to have_content('Quantity must be greater than 0')
  end
end
