# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Onboarding' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:admin) { users(:admin) }

  before do
    driven_by(:playwright)
    login_as(admin)
  end

  it 'allows setting up a medication with multiple dose options' do
    visit new_medication_path

    fill_in 'medication_name', with: 'Multi-dose Med'

    triggers = all('[data-ruby-ui--combobox-target="trigger"]')

    # Select Unit (index 2)
    triggers[2].click
    find('label', text: 'tablet').click
    page.send_keys(:escape)

    # Choose "No" for single dose
    find('label', text: 'No').click

    # Select some chips
    find('label', text: '0.5', exact_text: true).click
    find('label', text: '1', exact_text: true).click
    find('label', text: '2', exact_text: true).click

    # Add other dose
    fill_in 'medication_other_dosages', with: '5, 10'

    fill_in 'medication_current_supply', with: '100'

    click_button 'Save Medication'

    expect(page).to have_content('Multi-dose Med')

    # Check dosages on the show page
    expect(page).to have_content('0.5 tablet')
    expect(page).to have_content('1.0 tablet')
    expect(page).to have_content('2.0 tablet')
    expect(page).to have_content('5.0 tablet')
    expect(page).to have_content('10.0 tablet')
  end

  it 'allows setting up a custom dose in a schedule' do
    medication = create(:medication, name: 'Custom Dose Med', dosage_unit: 'ml', location: locations(:home))
    person = people(:john)

    visit schedules_workflow_path

    # Select type
    select 'OTC', from: 'schedule_type'

    # Select person
    select person.name, from: 'person_id'

    # Select medication
    select medication.name, from: 'medication_id'

    # Fill frequency in workflow
    fill_in 'frequency', with: 'Once a day'

    click_button 'Continue to schedule details'

    # Now on schedule form
    # Fill custom dose
    fill_in 'schedule_custom_dose_amount', with: '7.5'
    fill_in 'schedule_custom_dose_unit', with: 'ml'

    fill_in 'schedule_start_date', with: Time.zone.today.strftime('%Y-%m-%d')
    fill_in 'schedule_end_date', with: (Time.zone.today + 7.days).strftime('%Y-%m-%d')

    click_button 'Add Plan'

    expect(page).to have_content('Schedule was successfully created')
    expect(page).to have_content(person.name)
    expect(page).to have_content('Custom Dose Med')
    # Use uppercase because of CSS transform in the UI
    expect(page).to have_content(/7\.5\s?ML/)
  end
end
