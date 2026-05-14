# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenProductsFacts::Client do
  subject(:client) { described_class.new }

  describe '#product' do
    let(:barcode) { '5000436574637' }
    let(:api_url) { "https://world.openproductsfacts.org/api/v2/product/#{barcode}.json" }
    let(:default_fields) { 'product_name,generic_name,brands,quantity,categories_tags_en' }

    before { Rails.cache.clear }

    context 'when the product exists' do
      before do
        stub_request(:get, api_url)
          .with(
            headers: { 'Accept' => 'application/json', 'User-Agent' => 'MedTracker/1.0 (support@medtracker.app)' },
            query: hash_including('fields' => default_fields)
          )
          .to_return(
            status: 200,
            body: {
              'status' => 1,
              'code' => barcode,
              'product' => {
                'product_name' => 'Ibuprofen 200mg Pain Relief Tablets',
                'brands' => 'Tesco',
                'quantity' => '16 tablets'
              }
            }.to_json,
            headers: { 'Content-Type' => 'application/json' }
          )
      end

      it 'returns the product payload' do
        result = client.product(barcode)
        expect(result).to include('status' => 1)
        expect(result.dig('product', 'product_name')).to eq('Ibuprofen 200mg Pain Relief Tablets')
      end
    end

    context 'when the product does not exist' do
      before do
        stub_request(:get, api_url)
          .with(query: hash_including('fields' => default_fields))
          .to_return(
            status: 200,
            body: { 'status' => 0, 'code' => barcode }.to_json,
            headers: { 'Content-Type' => 'application/json' }
          )
      end

      it 'returns nil' do
        expect(client.product(barcode)).to be_nil
      end
    end

    context 'when a network error occurs' do
      before do
        stub_request(:get, api_url)
          .with(query: hash_including('fields' => default_fields))
          .to_raise(Net::ReadTimeout)
      end

      it 'raises ApiError' do
        expect { client.product(barcode) }.to raise_error(described_class::ApiError)
      end
    end

    context 'when the barcode is not numeric' do
      it 'returns nil without making a request' do
        expect(client.product('not-a-barcode')).to be_nil
        expect(WebMock).not_to have_requested(:get, /openproductsfacts/)
      end
    end
  end
end
