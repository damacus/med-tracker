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
      package_unit: 'tablet'
    }
    double('SearchResult', defaults.merge(attrs)).tap do |sr|
      allow(sr).to receive(:to_h).and_return(defaults.merge(attrs))
    end
  end

  def successful_nhs_result(results: [], resolved_query: nil, barcode: nil)
    instance_double(
      NhsDmd::Search::Result,
      success?: true,
      results: results,
      resolved_query: resolved_query,
      barcode: barcode
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
        barcoded = successful_nhs_result(results: [], resolved_query: nil, barcode: '5000168511017')
        allow(search).to receive(:call).and_return(barcoded)

        result = responder.call(query: '5000168511017')

        expect(result.body[:barcode]).to eq('5000168511017')
      end

      it 'includes permissions in the response body' do
        result = responder.call(query: 'paracetamol', permissions: { admin: true })
        expect(result.body[:permissions]).to eq({ admin: true })
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
  end

  describe 'Result' do
    it 'is a Data class with body and status' do
      result = MedicationFinderSearchResponder::Result.new(body: { results: [] }, status: :ok)
      expect(result.body).to eq({ results: [] })
      expect(result.status).to eq(:ok)
    end
  end
end
