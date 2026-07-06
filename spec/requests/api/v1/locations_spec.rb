# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 locations' do
  fixtures :accounts, :people, :users, :locations, :households

  let(:user) { users(:jane) }

  describe 'GET /api/v1/households/:household_id/locations' do
    it 'returns locations in the signed-in user scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_locations_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)

      returned_ids = response.parsed_body.fetch('data').map { |location| location.fetch('id') }
      expect(returned_ids).to include(locations(:home).id, locations(:school).id)
    end
  end

  describe 'GET /api/v1/households/:household_id/locations/:id' do
    it 'returns the location for the given id' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      location = locations(:home)

      get api_v1_household_location_path(household_id, location),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)

      returned_location = response.parsed_body.fetch('data')
      expect(returned_location.fetch('id')).to eq(location.id)
      expect(returned_location.fetch('name')).to eq(location.name)
      expect(returned_location.fetch('description')).to eq(location.description)
    end

    it 'returns not_found for a location outside scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      other_household = Household.create!(name: 'Other API Household', slug: 'other-api-household')
      other_location = Location.create!(name: 'Other', household: other_household)

      get api_v1_household_location_path(household_id, other_location),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:not_found)
    end
  end
end
