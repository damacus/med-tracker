# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Lookup', type: :system do
  fixtures :accounts, :people, :locations, :medications, :users

  let(:doctor) { users(:doctor) }

  let(:aspirin_results) do
    [
      {
        code: '39720311000001101',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        concept_class: 'VMP'
      },
      {
        code: '39720411000001102',
        display: 'Aspirin 75mg tablets',
        system: 'https://dmd.nhs.uk',
        concept_class: 'VMP'
      }
    ]
  end

  before do
    # Set credentials for NhsDmd::Client
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('NHS_DMD_CLIENT_ID', nil).and_return('test-id')
    allow(ENV).to receive(:fetch).with('NHS_DMD_CLIENT_SECRET', nil).and_return('test-secret')

    stub_nhs_dmd_token
  end

  it 'User searches for a medication and sees results' do
    stub_nhs_dmd_search(query: 'Aspirin', results: aspirin_results)

    sign_in(doctor)
    visit medication_finder_path

    expect(page).to have_content('Medication Finder')
    expect(page).to have_field('medication-search-input')
    expect(page).to have_button('Search')

    fill_in 'medication-search-input', with: 'Aspirin'
    click_button 'Search'

    expect(page).to have_content('Search Results')
    expect(page).to have_content('Aspirin 300mg tablets')
    expect(page).to have_content('Aspirin 75mg tablets')
    expect(page).to have_content('VMP')
  end

  it 'Search returns no results' do
    stub_nhs_dmd_search(query: 'NonExistentMedication12345', results: [])

    sign_in(doctor)
    visit medication_finder_path

    fill_in 'medication-search-input', with: 'NonExistentMedication12345'
    click_button 'Search'

    expect(page).to have_content('No medications found')
    expect(page).to have_content('Try searching with different terms')
  end

  it 'API unavailable shows error message' do
    # Stub search to return error status
    stub_request(:get, %r{#{NhsDmd::Client::BASE_URL}/ValueSet/\$expand})
      .to_return(status: 503, body: 'Service Unavailable')

    sign_in(doctor)
    visit medication_finder_path

    fill_in 'medication-search-input', with: 'Aspirin'
    click_button 'Search'

    expect(page).to have_content('Search unavailable')
  end

  it 'User views drug interactions (not yet implemented)' do
    pending 'MLKP-015: drug interaction lookup not yet implemented'

    sign_in(doctor)
    visit medication_finder_path

    fill_in 'medication-search-input', with: 'Warfarin'
    click_button 'Search'

    click_button('View Interaction Details', match: :first)

    expect(page).to have_content('Interaction Details')
    expect(page).to have_content('Severity')
    expect(page).to have_content('Description')
  end
end
