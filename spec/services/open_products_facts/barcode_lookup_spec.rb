# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenProductsFacts::BarcodeLookup do
  subject(:lookup) { described_class.new(client: client, audit_logger: audit_logger) }

  let(:client) { instance_double(OpenProductsFacts::Client) }
  let(:audit_logger) { instance_double(ExternalLookup::AuditLogger, record: nil) }
  let(:barcode) { '5000436574637' }

  let(:ibuprofen_product) do
    {
      'status' => 1,
      'code' => barcode,
      'product' => {
        'product_name' => 'Ibuprofen 200mg Pain Relief Tablets',
        'brands' => 'Tesco',
        'quantity' => '16 tablets'
      }
    }
  end

  it 'returns a catalog entry hash when the product is found' do
    allow(client).to receive(:product).with(barcode).and_return(ibuprofen_product)

    result = lookup.lookup(barcode)

    expect(result).to include(
      gtin: barcode,
      display: 'Ibuprofen 200mg Pain Relief Tablets (Tesco) 16 tablets',
      source: 'open_products_facts',
      concept_class: 'OTC Medicine'
    )
  end

  it 'persists the result to BarcodeCatalogEntry on the first lookup' do
    allow(client).to receive(:product).with(barcode).and_return(ibuprofen_product)

    expect { lookup.lookup(barcode) }
      .to change(BarcodeCatalogEntry, :count).by(1)

    entry = BarcodeCatalogEntry.find_by(gtin: barcode, source: 'open_products_facts')
    expect(entry).to be_present
    expect(entry.display).to eq('Ibuprofen 200mg Pain Relief Tablets (Tesco) 16 tablets')
  end

  it 'does not create a duplicate entry on repeated lookups' do
    allow(client).to receive(:product).with(barcode).and_return(ibuprofen_product)

    lookup.lookup(barcode)
    expect { lookup.lookup(barcode) }.not_to change(BarcodeCatalogEntry, :count)
  end

  it 'returns nil when the product is not found' do
    allow(client).to receive(:product).with(barcode).and_return(nil)

    expect(lookup.lookup(barcode)).to be_nil
  end

  it 'returns nil and logs a warning on API error' do
    allow(client).to receive(:product).with(barcode).and_raise(OpenProductsFacts::Client::ApiError, 'timeout')

    expect(Rails.logger).to receive(:warn).with(/OpenProductsFacts::BarcodeLookup failed/)
    expect(lookup.lookup(barcode)).to be_nil
  end

  it 'records an audit event on success' do
    allow(client).to receive(:product).with(barcode).and_return(ibuprofen_product)

    expect(audit_logger).to receive(:record).with(
      hash_including(source: 'open_products_facts', event: 'barcode_lookup', result_status: 'success')
    )

    lookup.lookup(barcode)
  end

  it 'records an audit event on not found' do
    allow(client).to receive(:product).with(barcode).and_return(nil)

    expect(audit_logger).to receive(:record).with(
      hash_including(source: 'open_products_facts', result_status: 'not_found')
    )

    lookup.lookup(barcode)
  end
end
