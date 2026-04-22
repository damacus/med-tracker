# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFoodFacts::Client do
  subject(:client) { described_class.new }

  describe '#product' do
    let(:barcode) { '5021265221301' }
    let(:response_body) do
      {
        code: barcode,
        status: 1,
        status_verbose: 'product found',
        product: {
          product_name: 'Wellman Original',
          brands: 'Vitabiotics',
          quantity: '30 tablets',
          categories_tags_en: %w[Supplements Vitamins],
          image_url: 'https://images.openfoodfacts.org/images/products/5021265221301/front.jpg'
        }
      }.to_json
    end

    before do
      Rails.cache.clear
      stub_request(:get, "https://world.openfoodfacts.org/api/v2/product/#{barcode}.json")
        .with(
          headers: {
            'Accept' => 'application/json',
            'User-Agent' => 'MedTracker/1.0 (support@medtracker.app)'
          },
          query: hash_including('fields' => 'product_name,brands,quantity,categories_tags_en,image_url')
        )
        .to_return(status: 200, body: response_body, headers: { 'Content-Type' => 'application/json' })
    end

    it 'returns the product payload when the barcode is known' do
      payload = client.product(barcode)

      expect(payload).to include(
        'code' => barcode,
        'status' => 1,
        'product' => hash_including(
          'product_name' => 'Wellman Original',
          'brands' => 'Vitabiotics',
          'quantity' => '30 tablets',
          'categories_tags_en' => %w[Supplements Vitamins]
        )
      )
    end

    it 'sends the required custom user agent' do
      client.product(barcode)

      expect(WebMock).to have_requested(:get, "https://world.openfoodfacts.org/api/v2/product/#{barcode}.json")
        .with(
          query: hash_including('fields' => 'product_name,brands,quantity,categories_tags_en,image_url'),
          headers: { 'User-Agent' => 'MedTracker/1.0 (support@medtracker.app)' }
        )
    end

    it 'returns nil when the product is not found' do
      stub_request(:get, 'https://world.openfoodfacts.org/api/v2/product/9999999999999.json')
        .with(query: hash_including('fields' => 'product_name,brands,quantity,categories_tags_en,image_url'))
        .to_return(
          status: 200,
          body: { code: '9999999999999', status: 0, status_verbose: 'product not found' }.to_json,
          headers: { 'Content-Type' => 'application/json' }
        )

      expect(client.product('9999999999999')).to be_nil
    end
  end

  describe '#search_products' do
    let(:response_body) do
      {
        count: 2,
        page: 1,
        page_size: 10,
        products: [
          {
            code: '5021265221301',
            product_name: 'Wellman Original',
            brands: 'Vitabiotics',
            quantity: '30 tablets',
            categories_tags_en: %w[Supplements Vitamins]
          },
          {
            code: '3017620422003',
            product_name: 'Nutella',
            brands: 'Ferrero',
            quantity: '400 g',
            categories_tags_en: ['Chocolate spreads']
          }
        ]
      }.to_json
    end

    before do
      Rails.cache.clear
      stub_request(:get, 'https://world.openfoodfacts.org/cgi/search.pl')
        .with(
          headers: {
            'Accept' => 'application/json',
            'User-Agent' => 'MedTracker/1.0 (support@medtracker.app)'
          },
          query: hash_including(
            'search_terms' => 'wellman',
            'search_simple' => '1',
            'action' => 'process',
            'json' => '1',
            'page_size' => '10'
          )
        )
        .to_return(status: 200, body: response_body, headers: { 'Content-Type' => 'application/json' })
    end

    it 'returns the products array for a full-text search' do
      payload = client.search_products('wellman')

      expect(payload).to contain_exactly(
        hash_including('code' => '5021265221301', 'product_name' => 'Wellman Original'),
        hash_including('code' => '3017620422003', 'product_name' => 'Nutella')
      )
    end
  end
end
