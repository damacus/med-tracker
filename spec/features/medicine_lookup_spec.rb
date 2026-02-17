# frozen_string_literal: true

require 'rails_helper'

RSpec.feature 'Medicine Lookup', type: :feature do
  fixtures :accounts, :people, :medicines, :users

  let(:doctor) { users(:doctor) }

    visit medicine_finder_path

    expect(page).to have_content('Medicine Finder')
    expect(page).to have_field('medicine-search-input')
    expect(page).to have_button('Search')

    fill_in 'medicine-search-input', with: 'Aspirin'
    click_button 'Search'

    # Should show search results
    expect(page).to have_content('Search Results')
    expect(page).to have_content('Aspirin')

    # Should show drug interactions section
    expect(page).to have_content('Drug Interactions')
  end

  scenario 'User views detailed interaction information' do
    login_as(create(:user, :doctor))

    visit medicine_finder_path

    fill_in 'medicine-search-input', with: 'Warfarin'
    click_button 'Search'

    # Click on a specific interaction
    click_button('View Interaction Details', match: :first)

    # Should show detailed interaction information
    expect(page).to have_content('Interaction Details')
    expect(page).to have_content('Severity')
    expect(page).to have_content('Description')
  end

  scenario 'Search returns no results' do
    login_as(create(:user, :doctor))

    visit medicine_finder_path

    fill_in 'medicine-search-input', with: 'NonExistentMedicine12345'
    click_button 'Search'

    expect(page).to have_content('No medicines found')
    expect(page).to have_content('Try searching with different terms')
  end
end
