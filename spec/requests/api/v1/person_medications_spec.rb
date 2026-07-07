# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 person medications' do
  fixtures :accounts, :people, :users, :households, :person_medications, :medications, :carer_relationships

  let(:user) { users(:admin) }
  let(:household_id) { user.person.household.id }

  describe 'GET /api/v1/households/:household_id/person_medications' do
    it 'returns person_medications' do
      login_data = api_login(user)
      get api_v1_household_person_medications_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)
      returned_ids = response.parsed_body.fetch('data').map { |pm| pm.fetch('id') }
      expect(returned_ids).to include(person_medications(:john_vitamin_d).id)
    end
  end

  describe 'GET /api/v1/households/:household_id/person_medications/:id' do
    it 'returns the person_medication' do
      login_data = api_login(user)
      person_medication = person_medications(:john_vitamin_d)

      get api_v1_household_person_medication_path(household_id, person_medication.id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(person_medication.id)
      expect(response.parsed_body.dig('data', 'administration_kind')).to eq('routine')
    end
  end

  describe 'POST /api/v1/households/:household_id/person_medications' do
    it 'creates a new person_medication' do
      login_data = api_login(user)
      person = people(:john)
      medication = medications(:ibuprofen)

      post api_v1_household_person_medications_path(household_id),
           params: {
             person_medication: {
               person_id: person.id,
               medication_id: medication.id,
               dose_amount: 200,
               dose_unit: 'mg',
               administration_kind: 'as_needed'
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'person_id')).to eq(person.id)
      expect(response.parsed_body.dig('data', 'medication_id')).to eq(medication.id)
      expect(response.parsed_body.dig('data', 'administration_kind')).to eq('as_needed')
    end

    it 'fails validation when creating duplicate person medication' do
      login_data = api_login(user)
      person = people(:john)
      medication = medications(:ibuprofen)

      post api_v1_household_person_medications_path(household_id),
           params: {
             person_medication: {
               person_id: person.id,
               medication_id: medication.id,
               dose_amount: 200,
               dose_unit: 'mg',
               administration_kind: 'as_needed'
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:created)

      post api_v1_household_person_medications_path(household_id),
           params: {
             person_medication: {
               person_id: person.id,
               medication_id: medication.id,
               dose_amount: 400,
               dose_unit: 'mg',
               administration_kind: 'routine'
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:unprocessable_content)
    end
  end

  describe 'PATCH /api/v1/households/:household_id/person_medications/:id' do
    it 'updates the person_medication' do
      login_data = api_login(user)
      person_medication = person_medications(:john_vitamin_d)

      patch api_v1_household_person_medication_path(household_id, person_medication.id),
            params: {
              person_medication: {
                notes: 'Use with food'
              }
            },
            headers: api_auth_headers(login_data.fetch('access_token')),
            as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'notes')).to eq('Use with food')
    end

    it 'fails validation when updating with invalid attributes' do
      login_data = api_login(user)
      person_medication = person_medications(:john_vitamin_d)

      patch api_v1_household_person_medication_path(household_id, person_medication.id),
            params: {
              person_medication: {
                dose_amount: -5
              }
            },
            headers: api_auth_headers(login_data.fetch('access_token')),
            as: :json

      expect(response).to have_http_status(:unprocessable_content)
    end
  end
end
