# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Inventory add schedule workflow', :browser do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  before do
    driven_by(:playwright)
    login_as(users(:admin))
  end

  it 'stays on inventory when cancelling before choosing a person' do
    visit medications_path

    within '[data-testid="medications-list"]' do
      click_on I18n.t('medications.index.add_for_person')
    end

    expect(page).to have_text('Who is this medication for?')

    click_button I18n.t('ruby_ui.common.close')

    expect(page).to have_current_path(medications_path)
    expect(page).to have_text('Inventory')
    expect(page).to have_no_text('Who is this medication for?')
  end

  it 'stays on inventory when cancelling after choosing a person' do
    visit medications_path

    within '[data-testid="medications-list"]' do
      click_on I18n.t('medications.index.add_for_person')
    end

    click_button 'Search people'
    find('[role="option"]', text: people(:john).name, wait: 10).click
    expect(page).to have_text("Add Medication for #{people(:john).name}")

    click_on 'Cancel'

    expect(page).to have_current_path(medications_path)
    expect(page).to have_text('Inventory')
    expect(page).to have_no_text("Add Medication for #{people(:john).name}")
  end

  it 'shows the saved medication on the person page after completing the standalone launcher' do
    person = people(:john)
    medication = medications(:paracetamol)

    visit add_medication_path(medication_id: medication.id)
    click_button 'Search people'
    find('[role="option"]', text: person.name, wait: 10).click

    expect(page).to have_text("Add Medication for #{person.name}")
    select '1000 mg - Standard adult dose', from: 'Dose'
    click_button 'Next'
    click_button 'Add Medication'

    expect(page).to have_current_path(person_path(person))
    expect(page).to have_text(I18n.t('person_medications.created'))
    expect(page).to have_text(medication.name)
    expect(page).to have_no_css('[role="dialog"]')
  end
end
