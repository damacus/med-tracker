# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Refill medication inventory' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before do
    driven_by(:playwright)
    sign_in(admin)
  end

  it 'shows refill actions on inventory pages' do
    visit medications_path
    expect(page).to have_text('Restock')

    visit location_path(medication.location)
    expect(page).to have_text('Restock')

    visit medication_path(medication)
    expect(page).to have_text('Restock')
  end

  it 'opens medication details from inventory card view action at the top level' do
    visit medications_path

    within "##{tenant_dom_id(medication)}" do
      click_link 'View'
    end

    expect(page).to have_current_path(medication_path(medication))
    expect(page).to have_css("[data-testid='medication-content']")
    expect(page).to have_text(medication.name)
  end

  it 'links medication cards on location pages to medication details' do
    visit location_path(medication.location)

    click_on medication.name

    expect(page).to have_current_path(medication_path(medication))
  end

  it 'refills supply from medication detail with quantity and restock date' do
    visit medication_path(medication)

    click_on 'Restock'

    expect(page).to have_field('refill_quantity')
    expect(page).to have_field('refill_restock_date', with: Date.current.to_s)

    fill_in 'refill_quantity', with: '12'
    fill_in 'refill_restock_date', with: Date.current.to_s
    click_on 'Refill'

    expect(page).to have_text('Inventory refilled successfully.')
    expect(page).to have_no_css('div[data-state="open"]')

    medication.reload
    expect(medication.current_supply).to eq(92)
  end

  it 'adds scanned stock from the inventory page' do
    medication.update!(barcode: '5012345678901', current_supply: 0, supply_at_last_restock: 30)

    visit medications_path
    click_on 'Scan stock'

    expect(page).to have_css('[data-testid="barcode-scanner"]')
    fill_in 'inventory_scan_barcode', with: '5012345678901'
    fill_in 'inventory_scan_quantity', with: '30'
    click_on 'Add scanned stock'

    expect(page).to have_text('Scanned stock added successfully.')
    expect(medication.reload.current_supply).to eq(30)
  end

  it 'shows matching stock details after barcode entry' do
    stub_inventory_scan_match(
      matched: true,
      medication: {
        name: medication.display_name,
        location: medication.location.name,
        current_supply: '15 units'
      }
    )

    visit medications_path
    click_on 'Scan stock'
    fill_in 'inventory_scan_barcode', with: '5012345678901'

    expect(page).to have_css('[data-testid="inventory-scan-match"]')
    within '[data-testid="inventory-scan-match"]' do
      expect(page).to have_text(medication.display_name)
      expect(page).to have_text(medication.location.name)
      expect(page).to have_text('15 units')
    end
    expect(page.evaluate_script('window.lastInventoryScanMatchUrl')).to end_with(
      '/medications/scan_restock_match.json?q=5012345678901'
    )
  end

  it 'shows when a barcode has no matching stock' do
    stub_inventory_scan_match(matched: false)

    visit medications_path
    click_on 'Scan stock'
    fill_in 'inventory_scan_barcode', with: '5012345678901'

    expect(page).to have_css('[data-testid="inventory-scan-no-match"]')
    expect(page).to have_text('No medication matched that barcode.')
  end

  it 'shows validation errors when refill quantity is invalid' do
    visit medication_path(medication)

    click_on 'Restock'
    fill_in 'refill_quantity', with: '0'
    fill_in 'refill_restock_date', with: Date.current.to_s

    click_on 'Refill'

    expect(page).to have_css('#refill_quantity:invalid')
  end

  def stub_inventory_scan_match(response)
    page.driver.with_playwright_page do |playwright_page|
      playwright_page.add_init_script(
        script: "window.inventoryScanMatchResponse = #{response.to_json};" \
                'window.fetch = async (url) => {' \
                'window.lastInventoryScanMatchUrl = url.toString();' \
                'return { ok: true, json: async () => window.inventoryScanMatchResponse };' \
                '};'
      )
    end
  end
end
