# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'GET /medicine-finder/search' do
  fixtures :accounts, :people, :users

  let(:doctor) { users(:doctor) }
  let(:doctor_account) { accounts(:dr_jones) }

  def login_as_doctor
    post '/login', params: { email: doctor_account.email, password: 'password' }
  end

  def login_as_carer
    post '/login', params: { email: accounts(:carer).email, password: 'password' }
  end

  describe 'GET /medicine-finder/search.json' do
    context 'with a valid query' do
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
      let(:search_outcome) { NhsDmd::Search::Result.new(results: search_results, error: nil) }

      before do
        login_as_doctor
        search = instance_double(NhsDmd::Search, call: search_outcome)
        allow(NhsDmd::Search).to receive(:new).and_return(search)
      end

      it 'returns 200 OK' do
        get medicine_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response).to have_http_status(:ok)
      end

      it 'returns JSON with results' do
        get medicine_finder_search_path(format: :json), params: { q: 'aspirin' }

        json = response.parsed_body
        expect(json['results']).to be_an(Array)
        expect(json['results'].first['display']).to eq('Aspirin 300mg tablets')
        expect(json['results'].first['code']).to eq('39720311000001101')
        expect(json['results'].first['concept_class']).to eq('VMP')
      end
    end

    context 'with a blank query' do
      before { login_as_doctor }

      it 'returns 200 with empty results' do
        get medicine_finder_search_path(format: :json), params: { q: '' }

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

      it 'returns 200 with an error message' do
        get medicine_finder_search_path(format: :json), params: { q: 'aspirin' }

        json = response.parsed_body
        expect(response).to have_http_status(:ok)
        expect(json['error']).to be_present
      end
    end

    context 'when the NHS dm+d service is not configured (credentials absent)' do
      let(:unconfigured_outcome) { NhsDmd::Search::Result.new(results: [], error: 'not_configured') }

      before do
        login_as_doctor
        search = instance_double(NhsDmd::Search, call: unconfigured_outcome)
        allow(NhsDmd::Search).to receive(:new).and_return(search)
      end

      it 'returns 200 with a not_configured error' do
        get medicine_finder_search_path(format: :json), params: { q: 'aspirin' }

        json = response.parsed_body
        expect(response).to have_http_status(:ok)
        expect(json['error']).to eq('not_configured')
      end
    end

    context 'when the user is not authenticated' do
      it 'redirects to login' do
        get medicine_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response).to redirect_to('/login')
      end
    end

    context 'when the user is a carer (not authorised)' do
      before { login_as_carer }

      it 'redirects with not authorized' do
        get medicine_finder_search_path(format: :json), params: { q: 'aspirin' }

        expect(response).to redirect_to(root_path)
      end
    end
  end
end
