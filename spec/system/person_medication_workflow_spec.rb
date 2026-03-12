# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Person medication workflow' do
  fixtures :accounts, :people, :locations, :medications, :users, :person_medications

  let(:person) { people(:child_user_person) }
  let(:parent) { users(:parent) }

  before do
    driven_by(:playwright)
    login_as(parent)
  end

  it 'lets a parent create and administer an as-needed medication for a linked child until the daily limit is hit' do
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_link 'Add Medication'
    end
    click_link 'As needed'

    expect(page).to have_button('Cancel')
    expect(page).to have_button('Next', disabled: true)
    expect(page).to have_no_button('Back')
    expect(page).to have_no_button('Add Medication')
    expect(page).to have_button('Next', disabled: true)

    click_button 'Select a medication'
    find('label', text: 'Calpol').click

    expect(page).to have_content('Choose the dose')
    expect(page).to have_button('Cancel')
    expect(page).to have_button('Back')
    expect(page).to have_button('Next', disabled: false)
    expect(page).to have_css('div.max-w-md')
    expect(page).to have_content('Medication')
    expect(page).to have_content('Calpol')

    expect(page).to have_no_content('Standard child dose')
    expect(page).to have_select('Dose', with_options: ['2.5 ml'])
    select '2.5 ml', from: 'Dose'
    click_button 'Next'

    expect(page).to have_content('Add optional guidance')
    expect(page).to have_content('Dose')
    expect(page).to have_content('2.5 ml')
    expect(page).to have_no_button('Next')
    expect(page).to have_button('Back')
    expect(page).to have_button('Cancel')
    expect(page).to have_button('Add Medication')

    fill_in 'person_medication_notes', with: 'Workflow test'
    fill_in 'person_medication_max_daily_doses', with: '1'

    click_button 'Add Medication'

    expect(page).to have_content('Medication added successfully')
    expect(page).to have_content('Workflow test')

    created_person_medication = person.person_medications.order(:id).last

    within("#person_medication_#{created_person_medication.id}") do
      expect(page).to have_button('💊 Give')
      click_button '💊 Give'
    end

    expect(page).to have_content('Medication taken successfully')

    within("#person_medication_#{created_person_medication.id}") do
      expect(page).to have_text(/today's doses/i)
      expect(page).to have_text(/2\.5ml/i)
      expect(page).to have_button('💊 Give', disabled: true)
      expect(page).to have_content('1/1')
    end
  end
end
