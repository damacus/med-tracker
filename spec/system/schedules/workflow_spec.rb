# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedules workflow' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  before do
    driven_by(:rack_test)
    sign_in(users(:admin))
  end

  it 'is discoverable from medication details' do
    visit medication_path(medications(:paracetamol))

    click_link 'Add Schedule'

    expect(page).to have_current_path(schedules_workflow_path(medication_id: medications(:paracetamol).id))
  end

  it 'routes workflow selections into a prefilled schedule form' do
    visit schedules_workflow_path

    select 'Prescribed', from: 'Type (OTC or prescribed)'
    select 'John Doe', from: 'Person name'
    select 'Paracetamol', from: 'Name of med'
    fill_in 'Dose, frequency', with: 'Twice daily'

    click_button 'Continue to schedule details'

    expect(page).to have_current_path(new_person_schedule_path(people(:john), medication_id: medications(:paracetamol).id, frequency: 'Twice daily', schedule_type: 'prescribed'))
    expect(page).to have_content('Add schedule for John Doe')
  end
end
