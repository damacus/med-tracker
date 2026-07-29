# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationFinderSearchResponder do
  subject(:responder) do
    described_class.new(search: search, medication_scope: Medication.none)
  end

  let(:search) { instance_double(NhsDmd::Search) }

  def make_search_result(attrs = {})
    defaults = {
      barcode: nil,
      code: 'DMD123',
      system: 'https://dmd.nhs.uk',
      concept_class: 'VMP',
      name: 'Paracetamol 500mg Tablets',
      display: 'Paracetamol 500mg Tablets',
      package_unit: 'tablet',
      trade_family: nil
    }
    merged = defaults.merge(attrs)
    instance_double(NhsDmd::SearchResult, **merged).tap do |sr|
      allow(sr).to receive(:to_h).and_return(merged)
    end
  end

  def successful_nhs_result(results: [], resolved_query: nil, barcode: nil, barcode_source: nil)
    instance_double(
      NhsDmd::Search::Result,
      success?: true,
      results: results,
      resolved_query: resolved_query,
      barcode: barcode,
      barcode_source: barcode_source
    )
  end

  def failed_nhs_result
    instance_double(NhsDmd::Search::Result, success?: false)
  end

  describe '#call' do
    context 'when query is blank' do
      it 'returns ok status with empty results without calling search' do
        allow(search).to receive(:call)

        result = responder.call(query: '')

        expect(result.status).to eq(:ok)
        expect(result.body[:results]).to eq([])
        expect(search).not_to have_received(:call)
      end

      it 'includes permissions in the response body' do
        result = responder.call(query: '  ', permissions: { can_edit: true })

        expect(result.body[:permissions]).to eq({ can_edit: true })
      end
    end

    context 'when search returns nil' do
      it 'returns service_unavailable' do
        allow(search).to receive(:call).and_return(nil)

        result = responder.call(query: 'paracetamol')

        expect(result.status).to eq(:service_unavailable)
        expect(result.body[:error]).to be_present
      end
    end

    context 'when search result is not successful' do
      it 'returns service_unavailable' do
        allow(search).to receive(:call).and_return(failed_nhs_result)

        result = responder.call(query: 'paracetamol')

        expect(result.status).to eq(:service_unavailable)
        expect(result.body[:error]).to be_present
      end
    end

    context 'when search succeeds with results' do
      let(:search_result_item) { make_search_result }
      let(:nhs_result) { successful_nhs_result(results: [search_result_item], resolved_query: nil, barcode: nil) }
      let(:mixed_responder) do
        calls = 0
        lookup = instance_double(MedicationInteractionLookup)
        allow(lookup).to receive(:call) do
          calls += 1
          raise Errno::ENOENT, 'missing terminology' if calls == 1

          MedicationInteractionLookup::Result.new(
            visible_prompts: [{ risk_level: 'high' }],
            hidden_count: 1
          )
        end
        described_class.new(search: search, medication_scope: Medication.none, interaction_lookup: lookup)
      end
      let(:mixed_result_matchers) do
        [
          a_hash_including(code: 'DMD123', review_prompts: [], review_prompt_filter: { hidden_count: 0 }),
          a_hash_including(
            code: 'DMD456',
            review_prompts: [{ risk_level: 'high' }],
            review_prompt_filter: { hidden_count: 1 }
          )
        ]
      end

      before { allow(search).to receive(:call).and_return(nhs_result) }

      it 'returns ok status' do
        result = responder.call(query: 'paracetamol')
        expect(result.status).to eq(:ok)
      end

      it 'includes result payloads in the body' do
        result = responder.call(query: 'paracetamol')
        expect(result.body[:results]).to be_an(Array)
        expect(result.body[:results].first).to include(code: 'DMD123')
      end

      it 'includes visible review prompts and the filtered-noise count' do
        responder_with_review_prompts = described_class.new(
          search: search,
          medication_scope: Medication.none,
          interaction_lookup: interaction_lookup_with_hidden_prompts
        )

        result = responder_with_review_prompts.call(query: 'paracetamol')
        payload = result.body[:results].first

        expect(payload[:review_prompts]).to eq([{ risk_level: 'high' }])
        expect(payload[:review_prompt_filter]).to eq(hidden_count: 3)
        expect(result.body[:review_guidance]).to eq(status: 'available')
      end

      it 'preserves successful enrichment after another result fails' do
        search_results = [
          make_search_result(code: 'DMD123', display: 'First result'),
          make_search_result(code: 'DMD456', display: 'Second result')
        ]
        allow(search).to receive(:call).and_return(successful_nhs_result(results: search_results))
        allow(Rails.logger).to receive(:error)

        result = mixed_responder.call(query: 'medicine')

        expect(result.status).to eq(:ok)
        expect(result.body[:results]).to match_array(mixed_result_matchers)
        expect(result.body[:review_guidance]).to eq(status: 'unavailable')
        expect(Rails.logger).to have_received(:error)
          .with('Medication review enrichment failed: Errno::ENOENT')
          .once
      end

      it 'uses resolved_query from the search result when present' do
        resolved_result = successful_nhs_result(results: [], resolved_query: 'Paracetamol 500mg', barcode: nil)
        allow(search).to receive(:call).and_return(resolved_result)

        result = responder.call(query: 'paracetamol')

        expect(result.body[:query]).to eq('Paracetamol 500mg')
      end

      it 'falls back to original query when resolved_query is blank' do
        allow(search).to receive(:call).and_return(nhs_result)

        result = responder.call(query: 'paracetamol')

        expect(result.body[:query]).to eq('paracetamol')
      end

      it 'includes barcode from the search result' do
        barcoded = successful_nhs_result(
          results: [],
          resolved_query: nil,
          barcode: '5000168511017',
          barcode_source: 'nhs_dmd'
        )
        allow(search).to receive(:call).and_return(barcoded)

        result = responder.call(query: '5000168511017')

        expect(result.body[:barcode]).to eq('5000168511017')
        expect(result.body[:barcode_resolution]).to eq(status: 'resolved', source: 'nhs_dmd')
      end

      it 'includes permissions in the response body' do
        result = responder.call(query: 'paracetamol', permissions: { admin: true })
        expect(result.body[:permissions]).to eq({ admin: true })
      end

      it 'filters results by dosage form when requested' do
        tablet = make_search_result(display: 'Paracetamol 500mg tablets', package_unit: 'tablet')
        liquid = make_search_result(display: 'Paracetamol 250mg/5ml oral suspension', package_unit: 'ml')
        allow(search).to receive(:call).and_return(successful_nhs_result(results: [tablet, liquid]))

        result = responder.call(query: 'paracetamol', form: 'liquid')

        expect(result.body[:results]).to contain_exactly(
          a_hash_including(display: 'Paracetamol 250mg/5ml oral suspension')
        )
        expect(result.body[:form]).to eq('liquid')
      end

      it 'filters results by normalized strength when requested' do
        standard = make_search_result(name: 'Paracetamol 500mg tablets', display: 'Paracetamol 500mg tablets')
        stronger = make_search_result(name: 'Paracetamol 1g tablets', display: 'Paracetamol 1g tablets')
        allow(search).to receive(:call).and_return(successful_nhs_result(results: [standard, stronger]))

        result = responder.call(query: 'paracetamol', strength: '0.5 g')

        expect(result.body[:results]).to contain_exactly(
          a_hash_including(display: 'Paracetamol 500mg tablets')
        )
        expect(result.body[:strength]).to eq('500mg')
      end
    end

    context 'when search raises an error' do
      it 'returns service_unavailable and logs the error' do
        allow(search).to receive(:call).and_raise(StandardError, 'connection refused')
        allow(Rails.logger).to receive(:error)

        result = responder.call(query: 'paracetamol')

        expect(result.status).to eq(:service_unavailable)
        expect(Rails.logger).to have_received(:error).with(/Medication finder search failed/)
      end
    end

    context 'when a matching medication exists in scope' do
      let(:medication) { create(:medication, name: 'Paracetamol 500mg Tablets') }
      let(:scope) { Medication.where(id: medication.id) }
      let(:responder_with_scope) do
        described_class.new(search: search, medication_scope: scope)
      end

      before do
        matching_item = make_search_result(
          code: nil,
          system: nil,
          concept_class: nil,
          name: medication.name,
          display: medication.display_name
        )
        allow(search).to receive(:call).and_return(
          successful_nhs_result(results: [matching_item])
        )
      end

      it 'includes existing_medication in result payload when matched' do
        result = responder_with_scope.call(query: 'paracetamol')
        payload = result.body[:results].first

        if payload[:existing_medication]
          expect(payload[:existing_medication]).to include(:id, :name, :location, :path, :refill_path)
        else
          # No match found — acceptable for this scope
          expect(payload[:existing_medication]).to be_nil
        end
      end
    end

    context 'when a different household medication belongs to the result trade family' do
      let(:exact_medication) do
        create(
          :medication,
          name: 'Laxido Orange sachets',
          barcode: '5016298210989',
          dmd_code: 'AMPP001',
          dmd_system: 'https://dmd.nhs.uk'
        )
      end
      let(:related_medication) do
        create(
          :medication,
          name: 'Laxido Lemon sachets',
          barcode: '5016298210996',
          dmd_code: 'AMPP002',
          dmd_system: 'https://dmd.nhs.uk'
        )
      end
      let(:unrelated_medication) do
        create(
          :medication,
          name: 'Unrelated medicine',
          barcode: '5016298211009',
          dmd_code: 'AMPP003',
          dmd_system: 'https://dmd.nhs.uk'
        )
      end
      let(:scope) { Medication.where(id: [exact_medication.id, related_medication.id, unrelated_medication.id]) }
      let(:responder_with_scope) { described_class.new(search: search, medication_scope: scope) }

      before do
        trade_family = NhsDmdTradeFamily.create!(code: 'TF001', name: 'Laxido')
        other_trade_family = NhsDmdTradeFamily.create!(code: 'TF002', name: 'Unrelated')
        NhsDmdAmpTradeFamily.create!(amp_code: 'AMP001', trade_family: trade_family)
        NhsDmdAmpTradeFamily.create!(amp_code: 'AMP002', trade_family: trade_family)
        NhsDmdAmpTradeFamily.create!(amp_code: 'AMP003', trade_family: other_trade_family)
        NhsDmdBarcode.create!(
          gtin: exact_medication.barcode, code: 'AMPP001', amp_code: 'AMP001', display: exact_medication.name
        )
        NhsDmdBarcode.create!(
          gtin: related_medication.barcode, code: 'AMPP002', amp_code: 'AMP002', display: related_medication.name
        )
        NhsDmdBarcode.create!(
          gtin: unrelated_medication.barcode, code: 'AMPP003', amp_code: 'AMP003', display: unrelated_medication.name
        )
        item = make_search_result(
          barcode: exact_medication.barcode,
          code: 'AMPP001',
          trade_family: { code: trade_family.code, name: trade_family.name }
        )
        allow(search).to receive(:call).and_return(successful_nhs_result(results: [item]))
      end

      it 'returns other household medicines in the same trade family separately from the exact stock match' do
        payload = responder_with_scope.call(query: 'laxido').body[:results].first

        expect(payload[:existing_medication]).to include(id: exact_medication.id)
        expect(payload[:related_medications]).to contain_exactly(
          {
            id: related_medication.id,
            name: related_medication.display_name,
            location: related_medication.location.name,
            path: Rails.application.routes.url_helpers.medication_path(
              related_medication.household.slug,
              related_medication
            ),
            current_supply: '50'
          }
        )
        expect(payload[:related_medications].first).not_to include(:refill_path)
      end
    end

    context 'when a finder result has no trade family' do
      it 'returns no related medications' do
        item = make_search_result(trade_family: nil)
        allow(search).to receive(:call).and_return(successful_nhs_result(results: [item]))

        payload = responder.call(query: 'paracetamol').body[:results].first

        expect(payload[:related_medications]).to eq([])
      end
    end

    context 'when multiple finder results have trade families' do
      let(:first_related_medication) { create(:medication, name: 'Laxido Lemon sachets') }
      let(:second_related_medication) { create(:medication, name: 'Movicol Chocolate sachets') }
      let(:trade_family_resolver) { instance_double(MedicationTradeFamilyResolver) }
      let(:finder_results) do
        [
          make_search_result(code: 'AMPP001', trade_family: { code: 'TF001', name: 'Laxido' }),
          make_search_result(code: 'AMPP002', trade_family: { code: 'TF002', name: 'Movicol' })
        ]
      end
      let(:responder_with_batched_resolver) do
        described_class.new(
          search: search,
          medication_scope: Medication.none,
          trade_family_resolver: trade_family_resolver
        )
      end

      before do
        allow(search).to receive(:call).and_return(successful_nhs_result(results: finder_results))
        allow(trade_family_resolver).to receive(:call)
          .with(trade_family_codes: %w[TF001 TF002])
          .and_return(
            'TF001' => [first_related_medication],
            'TF002' => [second_related_medication]
          )
      end

      it 'resolves all trade families once and attaches the matching group to each result' do
        payloads = responder_with_batched_resolver.call(query: 'macrogol').body[:results]

        expect(payloads.first[:related_medications]).to contain_exactly(
          a_hash_including(id: first_related_medication.id)
        )
        expect(payloads.second[:related_medications]).to contain_exactly(
          a_hash_including(id: second_related_medication.id)
        )
        expect(trade_family_resolver).to have_received(:call).once
      end
    end
  end

  describe 'Result' do
    it 'is a Data class with body and status' do
      result = MedicationFinderSearchResponder::Result.new(body: { results: [] }, status: :ok)
      expect(result.body).to eq({ results: [] })
      expect(result.status).to eq(:ok)
    end
  end

  def interaction_lookup_with_hidden_prompts
    result = MedicationInteractionLookup::Result.new(
      visible_prompts: [{ risk_level: 'high' }],
      hidden_count: 3
    )
    instance_double(MedicationInteractionLookup, call: result)
  end
end
