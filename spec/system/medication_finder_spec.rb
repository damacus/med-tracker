# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationFinder' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:user) { users(:john) }

  before do
    driven_by(:rack_test)
    login_as(user)
  end

  it 'displays the medication finder page' do
    visit medication_finder_path

    within '[data-testid="medication-finder"]' do
      aggregate_failures 'medication finder content' do
        expect(page).to have_text('Medication Finder')
        expect(page).to have_field('medication-search-input')
        expect(page).to have_select('medication-form-filter')
        expect(page).to have_button('Search')
        expected_text = 'Search NHS dm+d and supported product sources to add medication or restock existing inventory.'
        expect(page).to have_text(expected_text)
        expect(page).to have_css('[data-testid="barcode-scanner"]')
      end
    end
  end

  it 'opens a restock confirmation modal for an existing medication result' do
    driven_by(:playwright)
    login_as(user)
    medication = medications(:vitamin_c)
    medication.update!(current_supply: 10, supply_at_last_restock: 10)
    stub_medication_finder_search(medication)

    visit medication_finder_path
    fill_in 'medication-search-input', with: 'wellman'
    click_on 'Search'
    click_on 'Update stock'

    expect(page).to have_css('[data-testid="finder-restock-modal"]')
    expect(page).to have_text("Confirm you wish to add 30 units to #{medication.display_name}.")

    click_on 'Confirm restock'

    expect(page).to have_text('Inventory refilled successfully.')
    expect(medication.reload.current_supply).to eq(40)
  end

  it 'expands structured details for an external medicine result' do
    driven_by(:playwright)
    login_as(user)
    stub_medication_finder_payload(
      results: [
        {
          name: 'Aspirin 300mg tablets',
          display: 'Aspirin 300mg tablets',
          description: 'Pain relief medicine',
          directions: 'Take with water',
          warnings: 'Do not exceed the stated dose',
          category: 'Analgesic',
          package_size: '32 tablets',
          source_label: 'NHS dm+d'
        }
      ],
      permissions: { can_create: true, can_restock: true }
    )

    visit medication_finder_path
    fill_in 'medication-search-input', with: 'aspirin'
    click_on 'Search'
    click_on 'View medicine details'

    expect(page).to have_css('[data-testid="medicine-details"]')
    expect(page).to have_text('Pain relief medicine')
    expect(page).to have_text('Take with water')
    expect(page).to have_text('Do not exceed the stated dose')
  end

  def stub_medication_finder_search(medication)
    stub_medication_finder_payload(**medication_finder_payload(medication))
  end

  def stub_medication_finder_payload(payload = nil, **overrides)
    payload ||= overrides
    page.driver.with_playwright_page do |playwright_page|
      playwright_page.add_init_script(
        script: "window.medicationFinderPayload = #{payload.to_json};" \
                'window.fetch = async () => ({ ok: true, json: async () => window.medicationFinderPayload });'
      )
    end
  end

  def medication_finder_payload(medication)
    {
      results: [
        medication_finder_result(medication)
      ],
      permissions: { can_create: true, can_restock: true },
      barcode: '5021265221301'
    }
  end

  def medication_finder_result(medication)
    {
      name: 'Wellman Original',
      display: 'Wellman Original (Vitabiotics) 30 tablets',
      source_label: 'Open Food Facts',
      package_size: '30 tablets',
      package_quantity: 30,
      package_unit: 'tablet',
      existing_medication: medication_finder_existing_medication(medication)
    }
  end

  def medication_finder_existing_medication(medication)
    {
      id: medication.id,
      name: medication.display_name,
      location: medication.location.name,
      path: medication_path(medication),
      refill_path: refill_medication_path(medication),
      current_supply: '10'
    }
  end
end
