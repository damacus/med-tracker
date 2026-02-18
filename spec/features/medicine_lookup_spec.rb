# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medicine Lookup', type: :system do
  fixtures :accounts, :people, :medicines, :users

  let(:doctor) { users(:doctor) }

  let(:aspirin_results) do
    [
      NhsDmd::SearchResult.new(
        code: '39720311000001101',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        concept_class: 'VMP'
      ),
      NhsDmd::SearchResult.new(
        code: '39720411000001102',
        display: 'Aspirin 75mg tablets',
        system: 'https://dmd.nhs.uk',
        concept_class: 'VMP'
      )
    ]
  end

  it 'User searches for a medicine and sees results' do
    search = instance_double(NhsDmd::Search, call: NhsDmd::Search::Result.new(results: aspirin_results, error: nil))
    allow(NhsDmd::Search).to receive(:new).and_return(search)

    sign_in(doctor)
    visit medicine_finder_path

    expect(page).to have_content('Medicine Finder')
    expect(page).to have_field('medicine-search-input')
    expect(page).to have_button('Search')

    fill_in 'medicine-search-input', with: 'Aspirin'
    click_button 'Search'

    expect(page).to have_content('Search Results')
    expect(page).to have_content('Aspirin 300mg tablets')
    expect(page).to have_content('Aspirin 75mg tablets')
    expect(page).to have_content('VMP')
  end

  it 'Search returns no results' do
    search = instance_double(NhsDmd::Search, call: NhsDmd::Search::Result.new(results: [], error: nil))
    allow(NhsDmd::Search).to receive(:new).and_return(search)

    sign_in(doctor)
    visit medicine_finder_path

    fill_in 'medicine-search-input', with: 'NonExistentMedicine12345'
    click_button 'Search'

    expect(page).to have_content('No medicines found')
    expect(page).to have_content('Try searching with different terms')
  end

  it 'API unavailable shows error message' do
    search = instance_double(NhsDmd::Search,
                             call: NhsDmd::Search::Result.new(results: [], error: 'Service unavailable'))
    allow(NhsDmd::Search).to receive(:new).and_return(search)

    sign_in(doctor)
    visit medicine_finder_path

    fill_in 'medicine-search-input', with: 'Aspirin'
    click_button 'Search'

    expect(page).to have_content('Search unavailable')
  end

  it 'User views drug interactions (not yet implemented)' do
    pending 'MLKP-015: drug interaction lookup not yet implemented'

    sign_in(doctor)
    visit medicine_finder_path

    fill_in 'medicine-search-input', with: 'Warfarin'
    click_button 'Search'

    click_button('View Interaction Details', match: :first)

    expect(page).to have_content('Interaction Details')
    expect(page).to have_content('Severity')
    expect(page).to have_content('Description')
  end
end
