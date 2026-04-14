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
      click_link 'Add Medication'
    end
    click_link 'Prescribed / Scheduled'
    expect(page).to have_content("New Schedule for #{person.name}")
    expect(page).to have_content('Choose a medication')

    select 'Ibuprofen', from: 'Medication'

    choose('schedule_dose_option_200_mg', allow_label_click: true)

    fill_in 'Frequency', with: 'Twice daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')
    fill_in 'Notes', with: 'Turbo modal e2e schedule'

    expect(page).to have_button('Add Plan', disabled: false)
    click_button 'Add Plan'

    expect(page).to have_content('Schedule was successfully created.')
    expect(page).to have_no_content("New Schedule for #{person.name}")
    expect(page).to have_content('Ibuprofen')
  end
end
