# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::Search do
  subject(:search) do
    described_class.new(
      client: client,
      barcode_lookup: barcode_lookup,
      open_food_facts_lookup: open_food_facts_lookup,
      open_food_facts_search: open_food_facts_search
    )
  end

  let(:client) { instance_double(NhsDmd::Client) }
  let(:barcode_lookup) { instance_double(NhsDmd::BarcodeLookup, lookup: nil) }
  let(:open_food_facts_lookup) { instance_double(OpenFoodFacts::BarcodeLookup, lookup: nil) }
  let(:open_food_facts_search) { instance_double(OpenFoodFacts::Search, search: []) }

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

    context 'when the query is a known barcode' do
      before do
        barcode_result = {
          code: '13629411000001105',
          display: 'Laxido Orange oral powder sachets (Galen Ltd)',
          system: 'https://dmd.nhs.uk',
          concept_class: 'AMPP'
        }
        translated_results = [
          {
            code: '13629411000001105',
            display: 'Laxido Orange oral powder sachets (Galen Ltd)',
            system: 'https://dmd.nhs.uk',
            concept_class: 'AMPP'
          }
        ]

        allow(client).to receive(:configured?).and_return(true)
        allow(barcode_lookup).to receive(:lookup).with('5016298210989').and_return(barcode_result)
        allow(client).to receive(:search)
          .with('Laxido Orange oral powder sachets (Galen Ltd)')
          .and_return(translated_results)
      end

      it 'translates the barcode into a searchable dm+d query while preserving the scanned barcode' do
        result = search.call('5016298210989')

        expect(result).to be_success
        expect(result.resolved_query).to eq('Laxido Orange oral powder sachets (Galen Ltd)')
        expect(result.barcode).to eq('5016298210989')
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            code: '13629411000001105',
            display: 'Laxido Orange oral powder sachets (Galen Ltd)',
            concept_class: 'AMPP'
          )
        )
        expect(client).to have_received(:search).with('Laxido Orange oral powder sachets (Galen Ltd)')
      end
    end

    context 'when a barcode-resolved dm+d product has fuzzy sibling matches' do
      before do
        barcode_result = {
          code: '19736211000001105',
          display: 'Phenoxymethylpenicillin 125mg/5ml oral solution (Medreich Plc) 100 ml',
          system: 'https://dmd.nhs.uk',
          concept_class: 'AMPP'
        }
        translated_results = [
          barcode_result,
          {
            code: '23189811000001102',
            display: 'Flucloxacillin 125mg/5ml oral solution (Medreich Plc)',
            system: 'https://dmd.nhs.uk',
            concept_class: 'AMPP'
          },
          {
            code: '18719011000001104',
            display: 'Phenoxymethylpenicillin 125mg/5ml oral solution (Sigma Pharmaceuticals Plc)',
            system: 'https://dmd.nhs.uk',
            concept_class: 'AMPP'
          }
        ]

        allow(client).to receive(:configured?).and_return(true)
        allow(barcode_lookup).to receive(:lookup).with('5000123456789').and_return(barcode_result)
        allow(client).to receive(:search)
          .with('Phenoxymethylpenicillin 125mg/5ml oral solution (Medreich Plc) 100 ml')
          .and_return(translated_results)
      end

      it 'returns only the exact barcode match instead of the fuzzy sibling list' do
        result = search.call('5000123456789')

        expect(result).to be_success
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            code: '19736211000001105',
            display: 'Phenoxymethylpenicillin 125mg/5ml oral solution (Medreich Plc) 100 ml',
            match_reason: 'barcode_match'
          )
        )
      end
    end

    context 'when the barcode catalogue match needs dm+d enrichment by display name' do
      before do
        barcode_result = {
          display: 'Calprofen 100mg/5ml oral suspension',
          source: 'cd_data'
        }
        translated_results = [
          {
            code: '4585411000001109',
            display: 'Calprofen 100mg/5ml oral suspension (McNeil Products Ltd) 100 ml',
            system: 'https://dmd.nhs.uk',
            concept_class: 'AMPP'
          }
        ]

        allow(client).to receive(:configured?).and_return(true)
        allow(barcode_lookup).to receive(:lookup).with('3574661385488').and_return(barcode_result)
        allow(client).to receive(:search).with('Calprofen 100mg/5ml oral suspension').and_return(translated_results)
      end

      it 'returns enriched dm+d results keyed from the external barcode match' do
        result = search.call('3574661385488')

        expect(result).to be_success
        expect(result.resolved_query).to eq('Calprofen 100mg/5ml oral suspension')
        expect(result.barcode).to eq('3574661385488')
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            code: '4585411000001109',
            display: 'Calprofen 100mg/5ml oral suspension (McNeil Products Ltd) 100 ml',
            concept_class: 'AMPP'
          )
        )
      end
    end

    context 'when a barcode catalogue hit has no dm+d code and the NHS client is unavailable' do
      let(:barcode_result) do
        {
          display: 'Calprofen 100mg/5ml oral suspension',
          source: 'cd_data'
        }
      end

      before do
        allow(client).to receive(:configured?).and_return(false)
        allow(barcode_lookup).to receive(:lookup).with('3574661385488').and_return(barcode_result)
      end

      it 'still returns the barcode catalogue hit' do
        result = search.call('3574661385488')

        expect(result).to be_success
        expect(result.resolved_query).to eq('Calprofen 100mg/5ml oral suspension')
        expect(result.barcode).to eq('3574661385488')
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            code: nil,
            display: 'Calprofen 100mg/5ml oral suspension',
            concept_class: nil
          )
        )
      end
    end

    context 'when a barcode catalogue hit needs enrichment but the NHS API fails' do
      let(:barcode_result) do
        {
          display: 'Calprofen 100mg/5ml oral suspension',
          source: 'cd_data'
        }
      end

      before do
        allow(client).to receive(:configured?).and_return(true)
        allow(barcode_lookup).to receive(:lookup).with('3574661385488').and_return(barcode_result)
        allow(client).to receive(:search)
          .with('Calprofen 100mg/5ml oral suspension')
          .and_raise(NhsDmd::Client::ApiError, 'Service unavailable')
      end

      it 'falls back to the local barcode catalogue hit' do
        result = search.call('3574661385488')

        expect(result).to be_success
        expect(result.error).to be_nil
        expect(result.resolved_query).to eq('Calprofen 100mg/5ml oral suspension')
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            display: 'Calprofen 100mg/5ml oral suspension'
          )
        )
      end
    end

    context 'when the barcode is not in dm+d and Open Food Facts has a supplement match' do
      let(:off_result) do
        {
          display: 'Wellman Original (Vitabiotics) 30 tablets',
          barcode: '5021265221301',
          system: 'https://world.openfoodfacts.org',
          concept_class: 'Supplement',
          source: 'open_food_facts'
        }
      end

      before do
        allow(client).to receive(:configured?).and_return(false)
        allow(open_food_facts_lookup).to receive(:lookup).with('5021265221301').and_return(off_result)
      end

      it 'returns the Open Food Facts supplement match' do
        result = search.call('5021265221301')

        expect(result).to be_success
        expect(result.resolved_query).to eq('Wellman Original (Vitabiotics) 30 tablets')
        expect(result.barcode).to eq('5021265221301')
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            code: nil,
            barcode: '5021265221301',
            display: 'Wellman Original (Vitabiotics) 30 tablets',
            concept_class: 'Supplement'
          )
        )
      end
    end

    context 'when a known barcode resolves to an Open Food Facts supplement result' do
      let(:barcode_result) do
        {
          display: 'Wellman Original (Vitabiotics) 30 tablets',
          barcode: '5021265221301',
          system: 'https://world.openfoodfacts.org',
          concept_class: 'Supplement',
          source: 'open_food_facts'
        }
      end

      before do
        allow(client).to receive(:configured?).and_return(true)
        allow(barcode_lookup).to receive(:lookup).with('5021265221301').and_return(barcode_result)
        allow(open_food_facts_search).to receive(:search)
          .with('Wellman Original (Vitabiotics) 30 tablets')
          .and_return([barcode_result])
      end

      it 'keeps the supplement result on the secondary source instead of searching dm+d only' do
        result = search.call('5021265221301')

        expect(result).to be_success
        expect(result.resolved_query).to eq('Wellman Original (Vitabiotics) 30 tablets')
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            code: nil,
            barcode: '5021265221301',
            display: 'Wellman Original (Vitabiotics) 30 tablets',
            source_label: 'Open Food Facts'
          )
        )
      end
    end

    context 'when a known barcode resolves to a curated non-dm+d refill product' do
      let(:barcode_result) do
        {
          display: 'Calpol Vapour Plug & Nightlight + 3 Refill Pads',
          barcode: '3574661646435',
          system: 'Curated product catalog',
          concept_class: 'Accessory',
          source: 'curated'
        }
      end

      before do
        allow(client).to receive(:configured?).and_return(true)
        allow(client).to receive(:search)
        allow(barcode_lookup).to receive(:lookup).with('3574661646435').and_return(barcode_result)
      end

      it 'treats the curated barcode hit as authoritative instead of widening to fuzzy NHS results' do
        result = search.call('3574661646435')

        expect(result).to be_success
        expect(result.resolved_query).to eq('Calpol Vapour Plug & Nightlight + 3 Refill Pads')
        expect(result.barcode).to eq('3574661646435')
        expect(result.results.map(&:to_h)).to contain_exactly(
          a_hash_including(
            code: nil,
            barcode: '3574661646435',
            display: 'Calpol Vapour Plug & Nightlight + 3 Refill Pads',
            match_reason: 'barcode_match',
            source_label: 'Curated product catalog'
          )
        )
        expect(client).not_to have_received(:search)
      end
    end

    context 'when Open Food Facts has supplement text results for a non-medical query' do
      before do
        nhs_result = {
          code: '20370211000001107',
          display: 'Welland FreeStyle Vie belt large BLTL01 (Welland Medical Ltd)',
          system: 'https://dmd.nhs.uk',
          concept_class: 'AMPP'
        }
        off_result = {
          display: 'Wellman Original (Vitabiotics) 30 tablets',
          barcode: '5021265221301',
          system: 'https://world.openfoodfacts.org',
          concept_class: 'Supplement',
          source: 'open_food_facts'
        }

        allow(client).to receive(:configured?).and_return(true)
        allow(client).to receive(:search).with('wellman').and_return([nhs_result])
        allow(open_food_facts_search).to receive(:search).with('wellman').and_return([off_result])
      end

      it 'returns supplement results alongside NHS dm+d results with the supplement source first' do
        result = search.call('wellman')

        expect(result).to be_success
        expect(result.results.map(&:display)).to eq(
          [
            'Wellman Original (Vitabiotics) 30 tablets',
            'Welland FreeStyle Vie belt large BLTL01 (Welland Medical Ltd)'
          ]
        )
        expect(result.results.map(&:to_h)).to include(
          a_hash_including(barcode: '5021265221301', source_label: 'Open Food Facts'),
          a_hash_including(code: '20370211000001107', source_label: 'NHS dm+d')
        )
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
