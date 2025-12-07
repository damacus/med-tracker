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

    # RubyUI select widget for medicine
    find('button', text: 'Select a medicine').click
    find('div[role="option"]', text: 'Ibuprofen').click

    # Dosage options load after medicine selection
    find('button', text: 'Select a dosage').click
    expect(page).to have_selector('div[role="option"]', text: '400 mg - Standard adult dose')
    find('div[role="option"]', text: '400 mg - Standard adult dose').click

    fill_in 'Frequency', with: 'Once daily'
    fill_in 'Start date', with: Date.current
    fill_in 'End date', with: 1.week.from_now.to_date

    click_button 'Add Prescription'

    expect(page).to have_content('Prescription was successfully created.')
  end
end
