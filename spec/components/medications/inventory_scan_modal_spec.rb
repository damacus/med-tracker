# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::InventoryScanModal, type: :component do
  subject(:rendered) { render_inline(described_class.new) }

  it 'posts scanned stock to the scan restock endpoint' do
    form = rendered.at_css('form')

    expect(form['action']).to eq('/medications/scan_restock')
    expect(form['method']).to eq('post')
  end

  it 'wires barcode entry to the stock match endpoint' do
    controller = rendered.at_css('[data-controller="inventory-scan"]')
    barcode = rendered.at_css('#inventory_scan_barcode')

    expect(controller['data-inventory-scan-match-url-value']).to eq('/medications/scan_restock_match.json')
    expect(barcode['data-action']).to include('input->inventory-scan#barcodeChanged')
    expect(barcode['data-action']).to include('change->inventory-scan#barcodeChanged')
  end

  it 'renders hidden match and no-match regions for scan feedback' do
    match = rendered.at_css('[data-testid="inventory-scan-match"]')
    no_match = rendered.at_css('[data-testid="inventory-scan-no-match"]')

    expect(match.key?('hidden')).to be true
    expect(match['data-inventory-scan-target']).to include('matchPanel')
    expect(no_match.key?('hidden')).to be true
    expect(no_match['data-inventory-scan-target']).to include('noMatchPanel')
  end
end
