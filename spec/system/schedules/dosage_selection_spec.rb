# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedule dosage selection' do
  fixtures :accounts, :users, :people, :locations, :medications, :dosages

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:person) { people(:one) }

  it 'auto-advances to schedule details after medication selection' do
    login_as(admin)
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_on 'Add Medication'
    end
    click_on 'Prescribed / Scheduled'

    expect(page).to have_content('Choose a medication')

    find_by_id('medication_trigger').click
    # Portaled content is in body
    find('label', text: 'Ibuprofen', visible: :all, wait: 10).click

    expect(page).to have_content('Change')
    sleep 1.0 # Wait for dosage cards to render after medication selection

    expect(page).to have_no_css('[name="schedule[dose_option_key]"]:checked', visible: :hidden)
    expect(page).to have_content('Add Plan') # Button might be disabled, have_content is safer
    find('label', text: '400.0 mg', visible: :all, wait: 10).click

    fill_in 'Frequency', with: 'Once daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')

    click_on 'Add Plan'

    expect(page).to have_content('Schedule was successfully created.')
  end

  it 'shows a blocked dose step when the medication has no dosage choices' do
    Medication.create!(
      name: 'Dose-less medication',
      location: locations(:home),
      reorder_threshold: 0
    )

    login_as(admin)
    visit person_path(person)

    within '[data-testid="quick-actions"]' do
      click_on 'Add Medication'
    end
    click_on 'Prescribed / Scheduled'

    find_by_id('medication_trigger').click
    find('label', text: 'Dose-less medication', visible: :all, wait: 10).click

    expect(page).to have_content('No dose options are available for this medication.')
  end
end
