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

    # RubyUI Select for medicine - click trigger then option
    find('[data-testid="medicine-trigger"]').click
    find('div[role="option"]', text: 'Ibuprofen').click

    # Wait for dosage options to load dynamically
    expect(page).to have_css('[data-testid="dosage-trigger"]', wait: 5)

    # RubyUI Select for dosage - click the trigger using testid
    find('[data-testid="dosage-trigger"]').click
    expect(page).to have_selector('div[role="option"]', text: '400.0 mg - Standard adult dose')
    find('div[role="option"]', text: '400.0 mg - Standard adult dose').click

    fill_in 'Frequency', with: 'Once daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')

    # Wait for validation to enable the button
    expect(page).to have_button('Add Prescription', disabled: false)
    click_button 'Add Prescription'

    expect(page).to have_content('Prescription was successfully created.')
  end
end
