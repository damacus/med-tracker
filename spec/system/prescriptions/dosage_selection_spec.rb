# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Prescription dosage selection' do
  fixtures :accounts, :account_otp_keys, :users, :people, :medicines, :dosages

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:person) { people(:one) }

  it 'loads dosage options after selecting a medicine' do
    login_as(admin)
    visit person_path(person)

    click_link 'Add Prescription'

    # Native select for medicine
    select 'Ibuprofen', from: 'prescription_medicine_id'

    # Dosage options load dynamically after medicine selection
    # Wait for the dosage select to be populated
    expect(page).to have_select('prescription_dosage_id', with_options: ['400.0 mg - Standard adult dose'])
    select '400.0 mg - Standard adult dose', from: 'prescription_dosage_id'

    fill_in 'Frequency', with: 'Once daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')

    # Wait for validation to enable the button
    expect(page).to have_button('Add Prescription', disabled: false)
    click_button 'Add Prescription'

    expect(page).to have_content('Prescription was successfully created.')
  end
end
