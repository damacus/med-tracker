# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Inventory add schedule workflow', :browser do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  before do
    driven_by(:playwright)
    login_as(users(:admin))
  end

  it 'stays on inventory when cancelling before choosing a person' do
    visit medications_path

    within '[data-testid="medications-list"]' do
      click_on 'Add Schedule'
    end

    expect(page).to have_text('Who is this medication for?')

    click_on 'Close'

    expect(page).to have_current_path(medications_path)
    expect(page).to have_text('Inventory')
    expect(page).to have_no_text('Who is this medication for?')
  end

  it 'stays on inventory when cancelling after choosing a person' do
    visit medications_path

    within '[data-testid="medications-list"]' do
      click_on 'Add Schedule'
    end

    click_button 'Search people'
    find('[role="option"]', text: people(:john).name, wait: 10).click
    expect(page).to have_text("Add Medication for #{people(:john).name}")

    click_on 'Cancel'

    expect(page).to have_current_path(medications_path)
    expect(page).to have_text('Inventory')
    expect(page).to have_no_text("Add Medication for #{people(:john).name}")
  end
end
