# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedule dosage selection', :browser do
  fixtures :accounts, :users, :people, :locations, :medications, :dosages

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:person) { people(:one) }

  it 'auto-advances to schedule details after medication selection' do
    login_as(admin)
    visit new_person_schedule_path(person)

    expect(page).to have_text('Choose a medication')

    find_by_id('medication_trigger').click
    # Portaled content is in body
    find('[role="option"]', text: 'Ibuprofen', wait: 10).click

    expect(page).to have_text('Change')
    sleep 1.0 # Wait for dosage cards to render after medication selection

    expect(page).to have_no_css('[name="schedule[dose_option_key]"]:checked', visible: :hidden)
    expect(page).to have_text('Add Plan') # Button might be disabled, have_content is safer
    find('label', text: '400 mg', wait: 10).click

    fill_in 'Frequency', with: 'Once daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')

    click_on 'Add Plan'

    expect(page).to have_text('Schedule was successfully created.')
  end

  it 'shows a blocked dose step when the medication has no dosage choices' do
    Medication.create!(
      name: 'Dose-less medication',
      location: locations(:home),
      reorder_threshold: 0
    )

    login_as(admin)
    visit new_person_schedule_path(person)

    find_by_id('medication_trigger').click
    find('[role="option"]', text: 'Dose-less medication', wait: 10).click

    expect(page).to have_text('No dose options are available for this medication.')
  end

  it 'returns to the person page when cancelling the full-page form' do
    login_as(admin)
    visit new_person_schedule_path(person)

    click_on 'Cancel'

    expect(page).to have_current_path(person_path(person))
  end
end
