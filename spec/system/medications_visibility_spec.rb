# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationsVisibility' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  # admin is at Home location (admin_home fixture)
  let(:admin) { users(:admin) }
  # jane is a parent at Home location (jane_home fixture)
  let(:jane) { users(:jane) }

  before do
    driven_by(:playwright)
  end

  it 'allows a parent at the same Home location to see a medicine added by the admin' do
    # Step 1: Admin logs in and adds a new medication at Home
    login_as(admin)

    visit new_medication_path

    select 'Home', from: 'medication[location_id]'
    fill_in 'medication[name]', with: 'Test Medication E2E'
    select 'Painkiller', from: 'medication[category]'
    fill_in 'medication[dosage_amount]', with: '500'
    select 'mg', from: 'medication[dosage_unit]'
    fill_in 'medication[current_supply]', with: '50'

    click_button 'Save Medication'

    expect(page).to have_content('Test Medication E2E')
    expect(page).to have_content('Home')

    rodauth_logout

    # Step 2: Jane (parent, also at Home) logs in and should see the medication
    login_as(jane)

    visit medications_path

    expect(page).to have_content('Test Medication E2E')
  end
end
