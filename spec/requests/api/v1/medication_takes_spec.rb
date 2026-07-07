# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 medication takes' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications, :medication_takes, :carer_relationships

  let(:user) { users(:jane) }

  describe 'GET /api/v1/households/:household_id/medication_takes collection' do
    it 'returns only medication takes in the signed-in user scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medication_takes_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)

      returned_ids = response.parsed_body.fetch('data').map { |take| take.fetch('id') }
      expect(returned_ids).to include(medication_takes(:jane_morning_ibuprofen).id)
      expect(returned_ids).not_to include(medication_takes(:john_morning_paracetamol).id)
    end

    it 'applies valid collection filters and pagination bounds' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medication_takes_path(household_id),
          params: { updated_since: 1.year.ago.iso8601, page: 0, per_page: 500 },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('meta', 'page')).to eq(1)
      expect(response.parsed_body.dig('meta', 'per_page')).to eq(100)
    end

    it 'returns a structured error for an invalid updated_since filter' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medication_takes_path(household_id),
          params: { updated_since: 'not-a-timestamp' },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'code')).to eq('unprocessable_content')
    end
  end
end
