# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'GET /medication-finder/search' do
  fixtures :accounts, :people, :users, :locations, :medications, :location_memberships, :carer_relationships

  let(:doctor) { users(:doctor) }
  let(:doctor_account) { accounts(:dr_jones) }
  let(:admin) { users(:admin) }

  def login_as_admin
    sign_in(admin)
  end

  def login_as_doctor
    sign_in(doctor)
  end

  def login_as_carer
    sign_in(users(:carer))
  end

  def login_as_parent
    sign_in(users(:parent))
  end

  describe 'GET /medication-finder/search.json' do
    context 'with a valid query' do
      let(:search_results) do
        [
          NhsDmd::SearchResult.new(
            code: '39720311000001101',
            display: 'Aspirin 300mg tablets',
            system: 'https://dmd.nhs.uk',
            concept_class: 'VMP',
            pil_url: 'https://www.medicines.org.uk/emc/product/13866/pil'
          )
        ]
      end
      let(:search_outcome) { NhsDmd::Search::Result.new(results: search_results, error: nil) }

      before do
        login_as_admin
        search = instance_double(NhsDmd::Search, call: search_outcome)
        allow(NhsDmd::Search).to receive(:new).and_return(search)
      end

      it 'returns 200 OK' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response).to have_http_status(:ok)
      end

      it 'returns JSON with results' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        json = response.parsed_body
        expect(json['results']).to be_an(Array)
        expect(json['results'].first['display']).to eq('Aspirin 300mg tablets')
        expect(json['results'].first['code']).to eq('39720311000001101')
        expect(json['results'].first['concept_class']).to eq('VMP')
        expect(json['results'].first['pil_url']).to eq('https://www.medicines.org.uk/emc/product/13866/pil')
      end

      it 'includes existing medication metadata when the result matches accessible stock' do
        existing_medication = medications(:aspirin)
        existing_medication.update!(
          dmd_code: '39720311000001101',
          dmd_system: 'https://dmd.nhs.uk',
          dmd_concept_class: 'VMP'
        )

        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response.parsed_body['results'].first['existing_medication']).to include(
          'id' => existing_medication.id,
          'name' => existing_medication.display_name,
          'location' => 'Home',
          'path' => medication_path(existing_medication),
          'refill_path' => refill_medication_path(existing_medication),
          'current_supply' => '25'
        )
      end

      it 'includes finder action permissions for the current user' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response.parsed_body['permissions']).to include(
          'can_create' => true,
          'can_restock' => true
        )
      end

      it 'passes the selected dosage form filter to the responder' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin', form: 'tablet' }

        expect(response).to have_http_status(:ok)
        expect(response.parsed_body['form']).to eq('tablet')
      end

      it 'includes interaction warnings for matching accessible medication stock' do
        search = instance_double(
          NhsDmd::Search,
          call: NhsDmd::Search::Result.new(
            results: [
              NhsDmd::SearchResult.new(
                code: '3183411000001109',
                display: 'Warfarin 1mg tablets',
                system: 'https://dmd.nhs.uk',
                concept_class: 'VMP'
              )
            ],
            error: nil
          )
        )
        allow(NhsDmd::Search).to receive(:new).and_return(search)

        get medication_finder_search_path(format: :json), params: { q: 'warfarin' }

        expect(response).to have_http_status(:ok)
        expect(response.parsed_body.dig('results', 0, 'interactions')).to include(
          a_hash_including(
            'severity' => 'high',
            'severity_label' => 'High',
            'interacting_medication_name' => 'Ibuprofen'
          )
        )
      end
    end

    context 'when a result only matches inaccessible stock' do
      let(:search_results) do
        [
          NhsDmd::SearchResult.new(
            code: '99999911000001109',
            display: 'Foreign Search Only 123mg tablets',
            system: 'https://dmd.nhs.uk',
            concept_class: 'VMP'
          )
        ]
      end
      let(:search_outcome) { NhsDmd::Search::Result.new(results: search_results, error: nil) }

      before do
        login_as_parent
        foreign_household = Household.create!(name: 'Foreign Search Household', slug: 'foreign-search-household')
        foreign_location = Location.create!(name: 'Foreign', household: foreign_household)
        Medication.create!(
          name: 'Foreign Search Only',
          location: foreign_location,
          household: foreign_household,
          dose_amount: 123,
          dose_unit: 'mg',
          current_supply: 12,
          reorder_threshold: 2,
          dmd_code: '99999911000001109',
          dmd_system: 'https://dmd.nhs.uk',
          dmd_concept_class: 'VMP'
        )
        search = instance_double(NhsDmd::Search, call: search_outcome)
        allow(NhsDmd::Search).to receive(:new).and_return(search)
      end

      it 'does not expose existing medication metadata' do
        get medication_finder_search_path(format: :json), params: { q: 'foreign search only' }

        expect(response).to have_http_status(:ok)
        expect(response.parsed_body['results'].first).not_to have_key('existing_medication')
      end
    end

    context 'with a blank query' do
      before { login_as_doctor }

      it 'returns 200 with empty results' do
        get medication_finder_search_path(format: :json), params: { q: '' }

        json = response.parsed_body
        expect(response).to have_http_status(:ok)
        expect(json['results']).to eq([])
      end
    end

    context 'when the API is unavailable' do
      let(:error_outcome) { NhsDmd::Search::Result.new(results: [], error: 'Service unavailable') }

      before do
        login_as_doctor
        search = instance_double(NhsDmd::Search, call: error_outcome)
        allow(NhsDmd::Search).to receive(:new).and_return(search)
      end

      it 'returns 503 with a generic error message' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        json = response.parsed_body
        expect(response).to have_http_status(:service_unavailable)
        expect(json['error']).to eq('Medication search is temporarily unavailable.')
      end
    end

    context 'when the search service crashes unexpectedly' do
      before do
        login_as_doctor
        search = instance_double(NhsDmd::Search)
        allow(search).to receive(:call).and_raise(SocketError, 'lookup failed')
        allow(NhsDmd::Search).to receive(:new).and_return(search)
        allow(Rails.logger).to receive(:error)
      end

      it 'returns 503 instead of raising a server error' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response).to have_http_status(:service_unavailable)
        expect(response.parsed_body['error']).to eq('Medication search is temporarily unavailable.')
        expect(Rails.logger).to have_received(:error).with(/Medication finder search failed/)
      end
    end

    context 'when the NHS dm+d service is not configured (credentials absent)' do
      let(:unconfigured_outcome) { NhsDmd::Search::Result.new(results: [], error: 'not_configured') }

      before do
        login_as_doctor
        search = instance_double(NhsDmd::Search, call: unconfigured_outcome)
        allow(NhsDmd::Search).to receive(:new).and_return(search)
      end

      it 'returns 503 with a generic error message' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        json = response.parsed_body
        expect(response).to have_http_status(:service_unavailable)
        expect(json['error']).to eq('Medication search is temporarily unavailable.')
      end
    end

    context 'when the query is a locally imported barcode' do
      before do
        login_as_doctor
        NhsDmdBarcode.create!(
          gtin: '05016298210989',
          code: '13629411000001105',
          display: 'Laxido Orange oral powder sachets (Galen Ltd)',
          system: 'https://dmd.nhs.uk',
          concept_class: 'AMPP'
        )
        allow(ENV).to receive(:fetch).and_call_original
        allow(ENV).to receive(:fetch).with('NHS_DMD_CLIENT_ID', nil).and_return(nil)
        allow(ENV).to receive(:fetch).with('NHS_DMD_CLIENT_SECRET', nil).and_return(nil)
      end

      it 'returns the local barcode match without NHS API credentials' do
        get medication_finder_search_path(format: :json), params: { q: '5016298210989' }

        expect(response).to have_http_status(:ok)
        expect(response.parsed_body['results']).to contain_exactly(
          a_hash_including(
            'code' => '13629411000001105',
            'display' => 'Laxido Orange oral powder sachets (Galen Ltd)',
            'concept_class' => 'AMPP'
          )
        )
        expect(response.parsed_body['query']).to eq('Laxido Orange oral powder sachets (Galen Ltd)')
        expect(response.parsed_body['barcode']).to eq('5016298210989')
      end
    end

    context 'when the user is not authenticated' do
      it 'redirects to login' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response).to redirect_to('/login')
      end
    end

    context 'when the user is a carer who can restock' do
      let(:search_results) do
        [
          NhsDmd::SearchResult.new(
            code: '39720311000001101',
            display: 'Aspirin 300mg tablets',
            system: 'https://dmd.nhs.uk',
            concept_class: 'VMP'
          )
        ]
      end

      before do
        login_as_carer
        search = instance_double(
          NhsDmd::Search,
          call: NhsDmd::Search::Result.new(results: search_results, error: nil)
        )
        allow(NhsDmd::Search).to receive(:new).and_return(search)
      end

      it 'returns search results with restock-only permissions' do
        get medication_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response).to have_http_status(:ok)
        expect(response.parsed_body['permissions']).to include(
          'can_create' => false,
          'can_restock' => true
        )
      end
    end
  end
end
