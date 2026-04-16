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
    expect(page).to have_content('Restock')

    visit location_path(medication.location)
    expect(page).to have_content('Restock')

    visit medication_path(medication)
    expect(page).to have_content('Restock')
  end

  it 'links medication cards on location pages to medication details' do
    visit location_path(medication.location)

    click_on medication.name

    expect(page).to have_current_path(medication_path(medication))
  end

  it 'refills supply from medication detail with quantity and restock date' do
    visit medication_path(medication)

    click_on 'Restock'

    expect(page).to have_field('refill_quantity')
    expect(page).to have_field('refill_restock_date', with: Date.current.to_s)

    fill_in 'refill_quantity', with: '12'
    fill_in 'refill_restock_date', with: Date.current.to_s
    click_on 'Refill'

    expect(page).to have_content('Inventory refilled successfully.')
    expect(page).to have_no_css('div[data-state="open"]')

    medication.reload
    expect(medication.current_supply).to eq(92)
  end

  it 'shows validation errors when refill quantity is invalid' do
    visit medication_path(medication)

    click_on 'Restock'
    fill_in 'refill_quantity', with: '0'
    fill_in 'refill_restock_date', with: Date.current.to_s

    click_on 'Refill'

    expect(page).to have_css('#refill_quantity:invalid')
  end
end
