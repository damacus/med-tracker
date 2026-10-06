# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsWebsiteContent::Client do
  subject(:client) { described_class.new }

  let(:api_key) { 'test-api-key' }
  let(:cache_store) { ActiveSupport::Cache::MemoryStore.new }
  let(:base_url) { NhsWebsiteContent::Client::BASE_URL }

  before do
    allow(Rails).to receive(:cache).and_return(cache_store)
    Rails.cache.clear
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('NHS_WEBSITE_CONTENT_API_KEY', nil).and_return(api_key)
  end

  describe '#configured?' do
    context 'when the API key is present' do
      it 'returns true' do
        expect(client.configured?).to be true
      end
    end

    context 'when the API key is absent' do
      before { allow(ENV).to receive(:fetch).with('NHS_WEBSITE_CONTENT_API_KEY', nil).and_return(nil) }

      it 'returns false' do
        expect(client.configured?).to be false
      end
    end
  end

  describe '#list_medicines' do
    let(:medicines_response) do
      {
        'significantLink' => [
          {
            'name' => 'Paracetamol for adults',
            'url' => "#{base_url}/medicines/paracetamol-for-adults/"
          }
        ]
      }.to_json
    end

    context 'when the API returns a successful response' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines(\?.*)?$})
          .to_return(status: 200, body: medicines_response, headers: { 'Content-Type' => 'application/json' })
      end

      it 'returns parsed JSON for the medicines index' do
        result = client.list_medicines(category: 'P')

        expect(result['significantLink']).to be_an(Array)
        expect(result['significantLink'].first['name']).to eq('Paracetamol for adults')
      end

      it 'sends the apikey header' do
        client.list_medicines(category: 'P')

        expect(WebMock).to have_requested(:get, %r{nhs-website-content/medicines})
          .with(headers: { 'Apikey' => api_key })
      end

      it 'defaults page to "1"' do
        client.list_medicines(category: 'P')

        expect(WebMock).to have_requested(:get, %r{nhs-website-content/medicines})
          .with(query: hash_including('page' => '1'))
      end
    end

    context 'with a custom page' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines(\?.*)?$})
          .to_return(status: 200, body: '{}', headers: { 'Content-Type' => 'application/json' })
      end

      it 'passes the page parameter in the query string' do
        client.list_medicines(category: 'P', page: '2')

        expect(WebMock).to have_requested(:get, %r{nhs-website-content/medicines})
          .with(query: hash_including('page' => '2'))
      end
    end

    context 'when the response is not HTTP success' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines(\?.*)?$})
          .to_return(status: 404, body: 'Not Found')
      end

      it 'raises ApiError' do
        expect { client.list_medicines(category: 'P') }
          .to raise_error(described_class::ApiError, /404/)
      end
    end

    context 'when the response body is invalid JSON' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines(\?.*)?$})
          .to_return(status: 200, body: 'not json', headers: { 'Content-Type' => 'application/json' })
      end

      it 'raises ApiError mentioning invalid JSON' do
        expect { client.list_medicines(category: 'P') }
          .to raise_error(described_class::ApiError, /invalid JSON/)
      end
    end

    context 'when the request times out' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines(\?.*)?$}).to_timeout
      end

      it 'raises ApiError' do
        expect { client.list_medicines(category: 'P') }
          .to raise_error(described_class::ApiError, /NHS website API request failed/)
      end
    end

    context 'when the result is cached' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines(\?.*)?$})
          .to_return(status: 200, body: medicines_response, headers: { 'Content-Type' => 'application/json' })
      end

      it 'only makes one HTTP request for repeated calls with the same arguments' do
        client.list_medicines(category: 'P')
        client.list_medicines(category: 'P')

        expect(WebMock).to have_requested(:get, %r{nhs-website-content/medicines}).once
      end
    end
  end

  describe '#get_medicine' do
    let(:slug) { 'paracetamol-for-adults' }
    let(:medicine_response) do
      {
        'name' => 'Paracetamol for adults',
        'description' => 'Find out how paracetamol treats pain.',
        'webpage' => 'https://www.nhs.uk/medicines/paracetamol-for-adults/'
      }.to_json
    end

    context 'when the API returns a successful response' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines/#{slug}/})
          .to_return(status: 200, body: medicine_response, headers: { 'Content-Type' => 'application/json' })
      end

      it 'returns the parsed medicine JSON' do
        result = client.get_medicine(slug: slug)

        expect(result['name']).to eq('Paracetamol for adults')
      end

      it 'sends modules=true by default' do
        client.get_medicine(slug: slug)

        expect(WebMock).to have_requested(:get, %r{nhs-website-content/medicines/#{slug}/})
          .with(query: hash_including('modules' => 'true'))
      end
    end

    context 'when modules: false is passed' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines/#{slug}/})
          .to_return(status: 200, body: medicine_response, headers: { 'Content-Type' => 'application/json' })
      end

      it 'sends modules=false' do
        client.get_medicine(slug: slug, modules: false)

        expect(WebMock).to have_requested(:get, %r{nhs-website-content/medicines/#{slug}/})
          .with(query: hash_including('modules' => 'false'))
      end
    end

    context 'when the server returns 500' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines/#{slug}/})
          .to_return(status: 500, body: 'Internal Server Error')
      end

      it 'raises ApiError' do
        expect { client.get_medicine(slug: slug) }
          .to raise_error(described_class::ApiError, /500/)
      end
    end

    context 'when a network error occurs' do
      before do
        stub_request(:get, %r{nhs-website-content/medicines/#{slug}/})
          .to_raise(Errno::ECONNREFUSED)
      end

      it 'raises ApiError' do
        expect { client.get_medicine(slug: slug) }
          .to raise_error(described_class::ApiError, /NHS website API request failed/)
      end
    end
  end
end
