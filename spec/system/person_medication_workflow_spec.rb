# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medication workflow' do
  fixtures :accounts, :people, :locations, :location_memberships, :medications, :dosages, :users,
           :person_medications, :carer_relationships

  let(:person) { people(:child_user_person) }
  let(:parent) { users(:parent) }

  before do
    driven_by(:playwright)
    login_as(parent)
  end

  it 'lets a parent add a medication from predefined doses after choosing as-needed medication' do
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_link 'Add Medication'
    end

    expect(page).to have_text('Prescribed / Scheduled')
    expect(page).to have_text('How is this medication taken?')
    click_link 'As needed'

    expect(page).to have_text("Add Medication for #{person.name}")
    expect(page).to have_button('Cancel')
    expect(page).to have_button('Next', disabled: true)
    expect(page).to have_no_button('Back')
    expect(page).to have_no_button('Add Medication')

    click_button 'Select a medication'
    find('label', text: 'Paracetamol').click

    expect(page).to have_text('Choose the dose')
    expect(page).to have_button('Cancel')
    expect(page).to have_button('Back')
    expect(page).to have_button('Next')
    expect(page).to have_css('div.max-w-md')
    expect(page).to have_text('Medication')
    expect(page).to have_text('Paracetamol')

    expect(page).to have_select('Dose', with_options: ['250 mg - Standard child dose (6-12 years)'])
    select '250 mg - Standard child dose (6-12 years)', from: 'Dose'
    click_button 'Next'

    expect(page).to have_text('Review')
    expect(page).to have_text(/dose/i)
    expect(page).to have_text('250 mg')
    expect(page).to have_text('Every 4-6 hours')
    expect(page).to have_text('PLAN TYPE')
    expect(page).to have_text('As needed')
    expect(page).to have_no_text('Schedule type')
    expect(page).to have_no_text('Active dates')
    expect(page).to have_no_button('Next')
    expect(page).to have_button('Back')
    expect(page).to have_button('Cancel')
    expect(page).to have_button('Add Medication')

    click_button 'Add Medication'

    expect(page).to have_text('Medication added successfully.')
    expect(page).to have_no_text("Add Medication for #{person.name}")
    expect(page).to have_text(person.name)
    expect(page).to have_text('Paracetamol')

    created_medication = person.person_medications.order(:id).last
    expect(created_medication.administration_kind).to eq('as_needed')
    expect(created_medication.source_dosage_option).to eq(dosages(:paracetamol_child))
  end

  it 'does not leave a blank page when cancelled' do
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_link 'Add Medication'
    end

    expect(page).to have_text('How is this medication taken?')
    click_link 'As needed'

    expect(page).to have_text("Add Medication for #{person.name}")
    click_on 'Cancel'

    expect(page).to have_current_path(person_path(person))
    expect(page).to have_no_text("Add Medication for #{person.name}")
    expect(page).to have_text(person.name)
  end

  it 'shows zero minimum hours as a selected dose default' do
    medication = medications(:ibuprofen)

    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_link 'Add Medication'
    end

    click_link 'As needed'
    expect(page).to have_css("[data-controller~='person-medication-form']")
    click_button 'Select a medication'
    find('label', text: medication.name).click

    page.execute_script(<<~JS, medication.id)
      const form = document.querySelector("[data-controller~='person-medication-form']")
      const controller = window.Stimulus.getControllerForElementAndIdentifier(form, "person-medication-form")
      const options = controller.doseOptionsValue
      options[String(arguments[0])][0].default_min_hours_between_doses = 0
      controller.doseOptionsValue = options
      controller.updateDefaults()
    JS

    select '200 mg - Light adult dose', from: 'Dose'
    click_button 'Next'

    expect(page).to have_field('person_medication[min_hours_between_doses]', with: '0')
  end
end
