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

    # Step 1: Basic Info
    triggers = all('[data-ruby-ui--combobox-target="trigger"]')
    triggers[0].click  # location
    find('label', text: 'Home').click
    page.send_keys(:escape)

    fill_in 'medication_name', with: 'Test Medication E2E'

    triggers[1].click  # category
    find('label', text: 'Analgesic').click
    page.send_keys(:escape)

    click_button 'Continue'

    # Step 2: Dose & Schedule — wait for step to become visible
    expect(page).to have_field('Amount')
    fill_in 'Amount', with: '500'
    select 'mg', from: 'Unit'
    click_button 'Review dose schedule'

    click_button 'Continue'

    # Step 3: Supply
    expect(page).to have_field('medication_current_supply')
    fill_in 'medication_current_supply', with: '50'

    click_button 'Continue'

    # Step 4: Warnings — wait for step, then save
    expect(page).to have_button('Save Medication', visible: :visible)
    click_button 'Save Medication'

    # Step 5: Dosage wizard — finish setup to reach the medication detail page
    expect(page).to have_content('Test Medication E2E created!')
    click_link 'Done'

    expect(page).to have_content('Test Medication E2E')
    expect(page).to have_content('Home')

    rodauth_logout

    # Step 2: Jane (parent, also at Home) logs in and should see the medication
    login_as(jane)

    visit medications_path

    expect(page).to have_content('Test Medication E2E')
  end
end
