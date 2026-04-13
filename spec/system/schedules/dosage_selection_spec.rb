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
      click_link 'Add Medication'
    end
    click_link 'Prescribed / Scheduled'

    expect(page).to have_content('Choose a medication')

    select 'Ibuprofen', from: 'Medication'

    expect(page).to have_content('Change')
    expect(page).to have_no_css('[name="schedule[dose_option_key]"]:checked', visible: :hidden)
    expect(page).to have_button('Add Plan', disabled: true)
    choose('schedule_dose_option_400_mg', allow_label_click: true)

    fill_in 'Frequency', with: 'Once daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')

    expect(page).to have_button('Add Plan', disabled: false)
    click_button 'Add Plan'

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
      click_link 'Add Medication'
    end
    click_link 'Prescribed / Scheduled'

    select 'Dose-less medication', from: 'Medication'

    expect(page).to have_content('No dose options are available for this medication.')
    expect(page).to have_button('Add Plan', disabled: true)
  end
end
