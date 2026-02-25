# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medicines Authorization' do
  fixtures :accounts, :people, :users, :locations, :medicines, :dosages, :prescriptions

  before do
    driven_by(:rack_test)
  end

  it 'shows Add Medicine to administrators' do
    sign_in(users(:admin))

    visit medicines_path

    expect(page).to have_link('Add Medicine', href: new_medicine_path)
  end

  it 'shows Add Medicine to parents' do
    sign_in(users(:parent))

    visit medicines_path

    expect(page).to have_link('Add Medicine', href: new_medicine_path)
  end

  it 'does not show Add Medicine to nurses' do
    sign_in(users(:nurse))

    visit medicines_path

    expect(page).to have_content('Medicines')
    expect(page).to have_no_link('Add Medicine', href: new_medicine_path)
  end

  it 'does not show Add Medicine to carers' do
    sign_in(users(:carer))

    visit medicines_path

    expect(page).to have_content('Medicines')
    expect(page).to have_no_link('Add Medicine', href: new_medicine_path)
  end
end
