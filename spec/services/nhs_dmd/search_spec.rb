# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::Search do
  subject(:search) { described_class.new(client: client) }

  let(:client) { instance_double(NhsDmd::Client) }

  describe '#call' do
    context 'when the service is not configured (credentials absent)' do
      before do
        allow(client).to receive(:configured?).and_return(false)
        allow(client).to receive(:search)
      end

      it 'returns a not-configured error' do
        result = search.call('aspirin')

        expect(result).not_to be_success
        expect(result.error).to eq('not_configured')
      end
    end

    context 'when the query is blank' do
      before do
        allow(client).to receive(:configured?).and_return(true)
        allow(client).to receive(:search)
      end

      it 'returns a successful result with empty results' do
        result = search.call('')

        expect(result).to be_success
        expect(result.results).to eq([])
      end
    end

    context 'when the client returns results' do
      let(:raw_results) do
        [
          { code: '39720311000001101', display: 'Aspirin 300mg tablets', system: 'https://dmd.nhs.uk',
            concept_class: 'VMP' },
          { code: '12345678', display: 'Aspirin 75mg tablets', system: 'https://dmd.nhs.uk', concept_class: nil }
        ]
      end

      before do
        allow(client).to receive(:configured?).and_return(true)
        allow(client).to receive(:search).with('aspirin').and_return(raw_results)
      end

      it 'returns a successful result' do
        result = search.call('aspirin')

        expect(result).to be_success
        expect(result.error).to be_nil
      end

      it 'maps raw results to SearchResult objects' do
        result = search.call('aspirin')

        expect(result.results.length).to eq(2)
        expect(result.results.first).to be_a(NhsDmd::SearchResult)
      end

      it 'populates SearchResult attributes correctly' do
        result = search.call('aspirin')
        first = result.results.first

        expect(first.code).to eq('39720311000001101')
        expect(first.display).to eq('Aspirin 300mg tablets')
        expect(first.concept_class).to eq('VMP')
      end
    end

    context 'when the client raises an ApiError' do
      before do
        allow(client).to receive(:configured?).and_return(true)
        allow(client).to receive(:search).and_raise(NhsDmd::Client::ApiError, 'Service unavailable')
      end

      it 'returns a result with an error message' do
        result = search.call('aspirin')

        expect(result).not_to be_success
        expect(result.error).to include('Service unavailable')
      end

      it 'returns empty results on error' do
        result = search.call('aspirin')

        expect(result.results).to eq([])
      end
    end

    context 'when the client raises an unexpected error' do
      before do
        allow(client).to receive(:configured?).and_return(true)
        allow(client).to receive(:search).and_raise(SocketError, 'lookup failed')
        allow(Rails.logger).to receive(:error)
      end

      it 'returns a result instead of propagating the exception' do
        result = search.call('aspirin')

        expect(result).not_to be_success
        expect(result.error).to eq('unexpected_error')
        expect(result.results).to eq([])
        expect(Rails.logger).to have_received(:error).with(/NhsDmd::Search crashed/)
      end
    end
  end
end
