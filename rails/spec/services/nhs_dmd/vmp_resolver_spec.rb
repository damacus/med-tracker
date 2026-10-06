# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::VmpResolver do
  subject(:resolver) { described_class.new(client) }

  let(:client)     { instance_double(NhsDmd::Client, configured?: true) }
  let(:ampp_match) { { code: '123', display: 'Aspirin 300mg tablets (Brand Ltd)', concept_class: 'AMPP' } }
  let(:vmp_result) { { code: '789', display: 'Aspirin 300mg tablets', concept_class: 'VMP' } }
  let(:amp_result) { { code: '111', display: 'Aspirin 300mg tablets (Brand Ltd)', concept_class: 'AMP' } }

  before do
    allow(client).to receive(:search).and_return([amp_result, vmp_result])
  end

  describe '#resolve' do
    context 'when the client is not configured' do
      before { allow(client).to receive(:configured?).and_return(false) }

      it 'returns nil without calling the client' do
        result = resolver.resolve('Aspirin 300mg tablets (Brand Ltd)', ampp_match)

        expect(result).to be_nil
        expect(client).not_to have_received(:search)
      end
    end

    context 'when the barcode match is not AMP or AMPP' do
      let(:vmp_barcode_match) { { code: '789', concept_class: 'VMP' } }

      it 'returns nil' do
        result = resolver.resolve('Aspirin 300mg tablets', vmp_barcode_match)

        expect(result).to be_nil
      end
    end

    context 'when de-branding does not change the display name' do
      it 'returns nil because there is nothing to de-brand' do
        result = resolver.resolve('Aspirin 300mg tablets', ampp_match)

        expect(result).to be_nil
        expect(client).not_to have_received(:search)
      end
    end

    context 'when the barcode match is AMPP with a branded display' do
      it 'returns the first VMP result from the search' do
        result = resolver.resolve('Aspirin 300mg tablets (Brand Ltd)', ampp_match)

        expect(result).to eq(vmp_result)
      end

      it 'searches using the de-branded name' do
        resolver.resolve('Aspirin 300mg tablets (Brand Ltd)', ampp_match)

        expect(client).to have_received(:search).with('Aspirin 300mg tablets')
      end
    end

    context 'when the barcode match is AMP with a branded display' do
      it 'returns the first VMP result' do
        result = resolver.resolve('Aspirin 300mg tablets (Brand Ltd)',
                                  { code: '456', display: 'Aspirin 300mg tablets (Brand Ltd)', concept_class: 'AMP' })

        expect(result).to eq(vmp_result)
      end
    end

    context 'when no VMP result is returned by the client' do
      before do
        allow(client).to receive(:search).and_return([amp_result])
      end

      it 'returns nil' do
        result = resolver.resolve('Aspirin 300mg tablets (Brand Ltd)', ampp_match)

        expect(result).to be_nil
      end
    end

    context 'when the client raises an ApiError' do
      before do
        allow(client).to receive(:search).and_raise(NhsDmd::Client::ApiError, 'timeout')
      end

      it 'returns nil and does not re-raise' do
        expect(resolver.resolve('Aspirin 300mg tablets (Brand Ltd)', ampp_match)).to be_nil
      end
    end

    context 'when the client raises an unexpected StandardError' do
      before do
        allow(client).to receive(:search).and_raise(StandardError, 'unexpected')
      end

      it 'returns nil and does not re-raise' do
        expect(resolver.resolve('Aspirin 300mg tablets (Brand Ltd)', ampp_match)).to be_nil
      end
    end
  end
end
