# frozen_string_literal: true

require 'rails_helper'

RSpec.describe BarcodeCatalog::Resolver do
  subject(:resolver) { described_class.new(lookup: lookup) }

  let(:lookup) { instance_double(BarcodeCatalog::Lookup, lookup: nil) }

  it 'returns a resolved outcome with normalized barcode and source metadata' do
    allow(lookup).to receive(:lookup).with('5016298210989').and_return(
      display: 'Laxido Orange oral powder sachets',
      source: 'nhs_dmd'
    )

    result = resolver.call(' 5016-2982-10989 ')

    expect(result).to have_attributes(
      status: :resolved,
      barcode: '5016298210989',
      source: 'nhs_dmd',
      error: nil,
      match: a_hash_including(display: 'Laxido Orange oral powder sachets')
    )
  end

  it 'returns a not-found outcome for a valid unmapped barcode' do
    allow(lookup).to receive(:lookup).with('5016298210989').and_return(nil)

    expect(resolver.call('5016298210989')).to have_attributes(
      status: :not_found,
      barcode: '5016298210989',
      match: nil,
      source: nil,
      error: nil
    )
  end

  it 'returns an invalid outcome without invoking adapters' do
    expect(resolver.call('not-a-barcode')).to have_attributes(
      status: :invalid,
      barcode: '',
      match: nil,
      source: nil,
      error: 'invalid_barcode'
    )
    expect(lookup).not_to have_received(:lookup)
  end

  it 'returns a UI-safe error outcome when an adapter raises' do
    allow(lookup).to receive(:lookup).and_raise(Net::ReadTimeout, 'upstream details')
    allow(Rails.logger).to receive(:error)

    result = resolver.call('5016298210989')

    expect(result).to have_attributes(
      status: :error,
      barcode: '5016298210989',
      match: nil,
      source: nil,
      error: 'barcode_resolution_failed'
    )
    expect(Rails.logger).to have_received(:error).with(/BarcodeCatalog::Resolver failed: Net::ReadTimeout/)
  end
end
