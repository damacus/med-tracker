# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Add schedule modal flow' do
  fixtures :accounts, :users, :people, :locations, :medications, :dosages

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:person) { people(:child_patient) }

  it 'opens add schedule modal from person page and creates a schedule via turbo' do
    login_as(admin)
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_on 'Add Medication'
    end
    click_on 'Prescribed / Scheduled'
    expect(page).to have_content("New Schedule for #{person.name}")
    expect(page).to have_content('Choose a medication')

    find('#medication_trigger').click
    find('label', text: 'Ibuprofen', visible: :all, wait: 10).click

    sleep 1.0 # Wait for dosage cards to render
    find('label', text: /Standard child dose \(6-12 years\)/, visible: :all, wait: 10).click

    fill_in 'Frequency', with: 'Twice daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')
    fill_in 'Notes', with: 'Turbo modal e2e schedule'

    click_on 'Add Plan'

    expect(page).to have_content('Schedule was successfully created.')
    expect(page).to have_no_content("New Schedule for #{person.name}")
    expect(page).to have_content('Ibuprofen')
  end

  it 'closes the add schedule modal cleanly when cancelled from the details step' do
    login_as(admin)
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_on 'Add Medication'
    end
    click_on 'Prescribed / Scheduled'
    
    find('#medication_trigger').click
    find('label', text: 'Ibuprofen', visible: :all, wait: 10).click

    expect(page).to have_content("New Schedule for #{person.name}")
    expect(page).to have_content('Schedule details')

    click_on 'Cancel', match: :prefer_exact

    expect(page).to have_current_path(person_path(person))
    expect(page).to have_no_content("New Schedule for #{person.name}")
    expect(page).to have_content(person.name)
  end
end
