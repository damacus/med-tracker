# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedule dosage selection' do
  fixtures :accounts, :users, :people, :locations, :medications, :dosages

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:person) { people(:one) }

  it 'disables dosage select until medication is selected' do
    login_as(admin)
    visit person_path(person)

    click_link 'Add Schedule'

    # Dosage trigger should be disabled initially
    expect(page).to have_css('[data-testid="dosage-trigger"][disabled]')
    expect(page).to have_content('Select a medication first')

    # Select a medication
    find('[data-testid="medication-trigger"]').click
    find('div[role="option"]', text: 'Ibuprofen').click

    # Dosage trigger should now be enabled
    expect(page).to have_css('[data-testid="dosage-trigger"]:not([disabled])', wait: 5)

    # RubyUI Select for dosage - click the trigger
    find('[data-testid="dosage-trigger"]').click
    expect(page).to have_css('div[role="option"]', text: '400.0 mg - Standard adult dose')
    find('div[role="option"]', text: '400.0 mg - Standard adult dose').click

    fill_in 'Frequency', with: 'Once daily'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: 1.week.from_now.to_date.strftime('%Y-%m-%d')

    # Wait for validation to enable the button
    expect(page).to have_button('Add Schedule', disabled: false)
    click_button 'Add Schedule'

    expect(page).to have_content('Schedule was successfully created.')
  end
end
