# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications Authorization' do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  before do
    driven_by(:rack_test)
  end

  it 'shows Add Medication to administrators' do
    sign_in(users(:admin))

    visit medications_path

    expect(page).to have_link('Add Medication', href: new_medication_path)
  end

  it 'shows Add Medication to parents' do
    sign_in(users(:parent))

    visit medications_path

    expect(page).to have_link('Add Medication', href: new_medication_path)
  end

  it 'does not show Add Medication to nurses' do
    sign_in(users(:nurse))

    visit medications_path

    expect(page).to have_content('Medications')
    expect(page).to have_no_link('Add Medication', href: new_medication_path)
  end

  it 'does not show Add Medication to carers' do
    sign_in(users(:carer))

    visit medications_path

    expect(page).to have_content('Medications')
    expect(page).to have_no_link('Add Medication', href: new_medication_path)
  end
end
