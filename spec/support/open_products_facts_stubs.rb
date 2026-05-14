# frozen_string_literal: true

module OpenProductsFactsStubs
  OPF_PRODUCT_URL = %r{#{Regexp.escape(OpenProductsFacts::Client::BASE_URL)}/api/v2/product/}

  def stub_open_products_facts_not_found(barcode = nil)
    url = barcode ? "#{OpenProductsFacts::Client::BASE_URL}/api/v2/product/#{barcode}.json" : OPF_PRODUCT_URL
    stub_request(:get, url)
      .to_return(
        status: 200,
        body: { status: 0, status_verbose: 'product not found' }.to_json,
        headers: { 'Content-Type' => 'application/json' }
      )
  end

  def stub_open_products_facts_product(barcode, product_name:, brands: nil, quantity: nil)
    stub_request(:get, "#{OpenProductsFacts::Client::BASE_URL}/api/v2/product/#{barcode}.json")
      .with(query: hash_including('fields'))
      .to_return(
        status: 200,
        body: {
          status: 1,
          code: barcode,
          product: { product_name: product_name, brands: brands, quantity: quantity }.compact
        }.to_json,
        headers: { 'Content-Type' => 'application/json' }
      )
  end
end

RSpec.configure do |config|
  config.include OpenProductsFactsStubs

  # Stub OPf to return not-found by default so tests that don't care about it
  # aren't broken by the new barcode lookup fallback.
  config.before(:each, type: :request) do
    Rails.cache.clear
    stub_open_products_facts_not_found
  end
end
