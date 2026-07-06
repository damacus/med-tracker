# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 medications' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships, :medications

  let(:user) { users(:jane) }

  describe 'GET /api/v1/households/:household_id/medications collection' do
    it 'returns only medications in the signed-in user scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medications_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)

      returned_ids = response.parsed_body.fetch('data').map { |m| m.fetch('id') }
      expect(returned_ids).to include(medications(:paracetamol).id)
    end
  end

  describe 'GET /api/v1/households/:household_id/medications/:id' do
    it 'returns the medication when in scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medication_path(household_id, medications(:paracetamol).id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(medications(:paracetamol).id)
    end

    it 'returns not found for a medication outside scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      other_household = Household.create!(name: 'Other', slug: 'other')
      other_location = Location.create!(household: other_household, name: 'loc')
      other_medication = Medication.create!(household: other_household, location: other_location, name: 'Other')

      get api_v1_household_medication_path(household_id, other_medication.id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:not_found)
    end
  end

  describe 'POST /api/v1/households/:household_id/medications' do
    it 'creates a medication with valid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      post api_v1_household_medications_path(household_id),
           params: {
             medication: {
               name: 'New API Medication', location_id: locations(:home).id,
               dose_amount: 5, dose_unit: 'ml', current_supply: 100, reorder_threshold: 10
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'name')).to eq('New API Medication')
    end

    it 'returns unprocessable content with invalid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      post api_v1_household_medications_path(household_id),
           params: { medication: { name: '', location_id: locations(:home).id } },
           headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'errors')).to include('name')
    end
  end

  describe 'PATCH /api/v1/households/:household_id/medications/:id' do
    it 'updates a medication with valid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      patch api_v1_household_medication_path(household_id, medications(:paracetamol).id),
            params: { medication: { current_supply: 90 } },
            headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'current_supply')).to eq('90.0')
    end

    it 'returns unprocessable content with invalid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      patch api_v1_household_medication_path(household_id, medications(:paracetamol).id),
            params: { medication: { name: '' } },
            headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'errors')).to include('name')
    end
  end
end
