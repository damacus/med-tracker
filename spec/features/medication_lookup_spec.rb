# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication Lookup', type: :system do
  fixtures :accounts, :people, :locations, :medications, :users

  let(:admin) { users(:admin) }
  let(:doctor) { users(:doctor) }

  let(:aspirin_results) do
    [
      {
        code: '39720311000001101',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        concept_class: 'VMP',
        pil_url: 'https://www.medicines.org.uk/emc/product/13866/pil',
        spc_url: 'https://www.medicines.org.uk/emc/product/13866/smpc'
      },
      {
        code: '39720411000001102',
        display: 'Aspirin 75mg tablets',
        system: 'https://dmd.nhs.uk',
        concept_class: 'VMP'
      }
    ]
  end

  let(:barcode_results) do
    [
      {
        code: '13629411000001105',
        display: 'Laxido Orange oral powder sachets (Galen Ltd)',
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      }
    ]
  end

  let(:exact_barcode_result) do
    {
      code: '19736211000001105',
      display: 'Phenoxymethylpenicillin 125mg/5ml oral solution (Medreich Plc) 100 ml',
      system: 'https://dmd.nhs.uk',
      concept_class: 'AMPP'
    }
  end

  around do |example|
    previous_client_id = ENV.fetch('NHS_DMD_CLIENT_ID', nil)
    previous_client_secret = ENV.fetch('NHS_DMD_CLIENT_SECRET', nil)
    ENV['NHS_DMD_CLIENT_ID'] = 'test-id'
    ENV['NHS_DMD_CLIENT_SECRET'] = 'test-secret'
    Rails.cache.clear
    example.run
  ensure
    if previous_client_id.nil?
      ENV.delete('NHS_DMD_CLIENT_ID')
    else
      ENV['NHS_DMD_CLIENT_ID'] = previous_client_id
    end

    if previous_client_secret.nil?
      ENV.delete('NHS_DMD_CLIENT_SECRET')
    else
      ENV['NHS_DMD_CLIENT_SECRET'] = previous_client_secret
    end
    Rails.cache.clear
  end

  before do
    stub_nhs_dmd_token
  end

  it 'User searches for a medication and sees results' do
    stub_nhs_dmd_search(query: 'Aspirin', results: aspirin_results)

    sign_in(admin)
    visit medication_finder_path

    expect(page).to have_text('Medication Finder')
    expect(page).to have_field('medication-search-input')
    expect(page).to have_button('Search')

    fill_in 'medication-search-input', with: 'Aspirin'
    click_button 'Search'

    expect(page).to have_text('Search Results')
    expect(page).to have_text('Aspirin 300mg tablets')
    expect(page).to have_text('Aspirin 75mg tablets')
    expect(page).to have_text('VMP')
    expect_external_guidance_link(
      'pil-link',
      'https://www.medicines.org.uk/emc/product/13866/pil'
    )
    expect_external_guidance_link(
      'spc-link',
      'https://www.medicines.org.uk/emc/product/13866/smpc'
    )
  end

  it 'Search returns no results' do
    stub_nhs_dmd_search(query: 'NonExistentMedication12345', results: [])

    sign_in(admin)
    visit medication_finder_path

    fill_in 'medication-search-input', with: 'NonExistentMedication12345'
    click_button 'Search'

    expect(page).to have_text('No medications found')
    expect(page).to have_text('Try searching with different terms')
  end

  it 'API unavailable shows error message' do
    # Stub search to return error status
    stub_request(:get, %r{#{NhsDmd::Client::BASE_URL}/ValueSet/\$expand})
      .to_return(status: 503, body: 'Service Unavailable')

    sign_in(admin)
    visit medication_finder_path

    fill_in 'medication-search-input', with: 'Aspirin'
    click_button 'Search'

    expect(page).to have_text('Search unavailable')
  end

  it 'allows selecting a search result and prefill a new inventory item' do
    stub_nhs_dmd_search(query: '5016298210989', results: barcode_results)

    sign_in(admin)
    visit medication_finder_path

    fill_in 'medication-search-input', with: '5016298210989'
    click_button 'Search'
    within('[data-testid="result-card"]', match: :first) do
      click_link 'Add Medication'
    end

    expect(page).to have_current_path(%r{/medications/new})
    expect(page).to have_field('medication_name', with: 'Laxido Orange oral powder sachets (Galen Ltd)')
    expect(page).to have_field('medication[barcode]', with: '5016298210989', type: :hidden)
  end

  it 'translates an imported barcode into a searchable dm+d term in the finder UI' do
    NhsDmdBarcode.create!(
      gtin: '05016298210989',
      code: '13629411000001105',
      display: 'Laxido Orange oral powder sachets (Galen Ltd)',
      system: 'https://dmd.nhs.uk',
      concept_class: 'AMPP'
    )
    stub_nhs_dmd_search(query: 'Laxido Orange oral powder sachets (Galen Ltd)', results: barcode_results)
    stub_nhs_dmd_search(
      query: 'Laxido Orange oral powder sachets',
      results: [{ code: '13629311000001101', display: 'Laxido Orange oral powder sachets',
                  system: 'https://dmd.nhs.uk', concept_class: 'VMP' }]
    )

    sign_in(admin)
    visit medication_finder_path

    fill_in 'medication-search-input', with: '5016298210989'
    click_button 'Search'

    expect(page).to have_field('medication-search-input', with: '5016298210989')
    expect(page).to have_css(
      '[data-testid="search-results-header"]',
      text: '5016298210989'
    )
    expect(page).to have_no_css(
      '[data-testid="search-results-header"]',
      text: 'Laxido Orange oral powder sachets (Galen Ltd)'
    )
    expect(page).to have_text('Barcode match')

    within('[data-testid="result-card"]', match: :first) do
      click_link 'Add Medication'
    end

    expect(page).to have_current_path(%r{/medications/new})
    expect(page).to have_field('medication_name', with: 'Laxido Orange oral powder sachets')
    expect(page).to have_field('medication[barcode]', with: '5016298210989', type: :hidden)
  end

  it 'shows the exact scanned match instead of a fuzzy barcode sibling list' do
    NhsDmdBarcode.create!(
      gtin: '05000123456789',
      code: '19736211000001105',
      display: 'Phenoxymethylpenicillin 125mg/5ml oral solution (Medreich Plc) 100 ml',
      system: 'https://dmd.nhs.uk',
      concept_class: 'AMPP'
    )
    stub_nhs_dmd_search(
      query: 'Phenoxymethylpenicillin 125mg/5ml oral solution (Medreich Plc) 100 ml',
      results: [
        exact_barcode_result,
        {
          code: '23189811000001102',
          display: 'Flucloxacillin 125mg/5ml oral solution (Medreich Plc)',
          system: 'https://dmd.nhs.uk',
          concept_class: 'AMPP'
        },
        {
          code: '18719011000001104',
          display: 'Phenoxymethylpenicillin 125mg/5ml oral solution (Sigma Pharmaceuticals Plc)',
          system: 'https://dmd.nhs.uk',
          concept_class: 'AMPP'
        }
      ]
    )

    sign_in(admin)
    visit medication_finder_path

    fill_in 'medication-search-input', with: '5000123456789'
    click_button 'Search'

    expect(page).to have_text('Phenoxymethylpenicillin 125mg/5ml oral solution (Medreich Plc) 100 ml')
    expect(page).to have_text('Barcode match')
    expect(page).to have_no_text('Flucloxacillin 125mg/5ml oral solution (Medreich Plc)')
    expect(page).to have_no_text('Phenoxymethylpenicillin 125mg/5ml oral solution (Sigma Pharmaceuticals Plc)')
  end

  it 'does not persist a non-GTIN numeric query as a barcode' do
    stub_nhs_dmd_search(
      query: '1234567890',
      results: [
        {
          code: '99999999999999999',
          display: 'Cetirizine 10mg tablets',
          system: 'https://dmd.nhs.uk',
          concept_class: 'VMP'
        }
      ]
    )

    sign_in(admin)
    visit medication_finder_path

    fill_in 'medication-search-input', with: '1234567890'
    click_button 'Search'
    within('[data-testid="result-card"]', match: :first) do
      click_link 'Add Medication'
    end

    expect(page).to have_current_path(%r{/medications/new})
    expect(page).to have_field('medication_name', with: 'Cetirizine 10mg tablets')
    expect(page).to have_no_field('medication[barcode]', type: :hidden)
  end

  it 'User views medication review prompts from lookup results' do
    stub_nhs_dmd_search(
      query: 'Warfarin',
      results: [
        {
          code: '3183411000001109',
          display: 'Warfarin 1mg tablets',
          system: 'https://dmd.nhs.uk',
          concept_class: 'VMP'
        }
      ]
    )

    sign_in(admin)
    visit medication_finder_path

    fill_in 'medication-search-input', with: 'Warfarin'
    click_button 'Search'

    expect(page).to have_text('2 medication review prompts (High)')

    click_button('View Review Prompt Details', match: :first)

    expect(page).to have_text('Review Prompt Details')
    expect(page).to have_text('Review with a practitioner')
    expect(page).to have_text('Risk level')
    expect(page).to have_text('High')
    expect(page).to have_text('Ibuprofen')
    expect(page).to have_text('pharmacist, nurse, GP, or prescriber')
    expect(page).to have_no_text('interaction warning')
  end

  def expect_external_guidance_link(test_id, href)
    expect(page).to have_css(
      "a[data-testid=\"#{test_id}\"][href=\"#{href}\"][target=\"_blank\"][rel~=\"noopener\"][rel~=\"noreferrer\"]"
    )
  end
end
