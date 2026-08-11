# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedules workflow' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  before do |example|
    driven_by(example.metadata[:js] ? :playwright : :rack_test)
    sign_in(users(:admin))
  end

  it 'is discoverable from medication details' do
    visit medication_path(medications(:paracetamol))

    click_on 'Schedule'

    expect(page).to have_current_path(add_medication_path(medication_id: medications(:paracetamol).id))
  end

  it 'routes workflow selections into a prefilled schedule form' do
    visit schedules_workflow_path

    select 'Prescribed', from: 'Type (OTC or prescribed)'
    select 'John Doe', from: 'Person name'
    select 'Paracetamol', from: 'Name of med'
    fill_in 'Dose, frequency', with: 'Twice daily'

    click_button 'Continue to schedule details'

    expect(page).to have_current_path(
      new_person_schedule_path(
        people(:john),
        medication_id: medications(:paracetamol).id,
        frequency: 'Twice daily',
        schedule_type: 'prescribed'
      )
    )
    expect(page).to have_text('Add schedule for John Doe')
  end

  it 'keeps native invalid feedback for a required field without an error target', :js do
    visit schedules_workflow_path

    click_button 'Continue to schedule details'
    invalid_event_completed = page.evaluate_script(<<~JS)
      document.querySelector('#person_id').dispatchEvent(
        new Event('invalid', { cancelable: true })
      )
    JS

    aggregate_failures do
      expect(invalid_event_completed).to be(true)
      expect(page.evaluate_script('document.querySelector("#person_id").matches(":invalid")')).to be(true)
      expect(page).to have_no_css('#person_id[aria-invalid]')
    end
  end
end
