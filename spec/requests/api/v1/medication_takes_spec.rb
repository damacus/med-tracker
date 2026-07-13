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

  describe 'POST /api/v1/households/:household_id/medication_takes' do
    it 'records takes from person medication sources' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      person_medication = person_medications(:jane_vitamin_d)

      post api_v1_household_medication_takes_path(household_id),
           params: {
             medication_take: {
               client_uuid: SecureRandom.uuid,
               source_type: 'person_medication',
               source_id: person_medication.portable_id,
               taken_at: Time.current.iso8601
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'person_medication_portable_id')).to eq(person_medication.portable_id)
    end

    it 'returns not found for unknown source types' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      post api_v1_household_medication_takes_path(household_id),
           params: {
             medication_take: {
               client_uuid: SecureRandom.uuid,
               source_type: 'unknown',
               source_id: schedules(:jane_ibuprofen).portable_id,
               taken_at: Time.current.iso8601
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:not_found)
    end

    it 'returns unprocessable content for invalid taken_at values' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      post api_v1_household_medication_takes_path(household_id),
           params: {
             medication_take: {
               client_uuid: SecureRandom.uuid,
               source_type: 'schedule',
               source_id: schedules(:jane_ibuprofen).portable_id,
               taken_at: 'not-a-time'
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'message')).to eq('taken_at is invalid')
    end

    it 'returns existing takes for repeated client UUIDs' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      existing_take = medication_takes(:jane_morning_ibuprofen)

      post api_v1_household_medication_takes_path(household_id),
           params: {
             medication_take: {
               client_uuid: existing_take.client_uuid,
               source_type: 'schedule',
               source_id: schedules(:jane_ibuprofen).portable_id,
               taken_at: Time.current.iso8601
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(existing_take.id)
    end

    it 'records takes without idempotency client UUIDs' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      service = instance_double(TakeMedicationService)
      take = create(
        :medication_take,
        schedule: schedules(:jane_ibuprofen),
        person_medication: nil,
        skip_stock_mutation: true
      )
      allow(TakeMedicationService).to receive(:new).and_return(service)
      allow(service).to receive(:call).and_return(TakeMedicationService::Result.new(true, take, nil))

      post api_v1_household_medication_takes_path(household_id),
           params: {
             medication_take: {
               client_uuid: '',
               source_type: 'schedule',
               source_id: schedules(:jane_ibuprofen).portable_id,
               taken_at: Time.current.iso8601
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')),
           as: :json

      expect(response).to have_http_status(:created)
      expect(service).to have_received(:call).with(hash_including(client_uuid: ''))
    end

    it 'returns API failure messages from medication take service errors' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      service = instance_double(TakeMedicationService)
      allow(TakeMedicationService).to receive(:new).and_return(service)
      expected_messages = {
        out_of_stock: 'Cannot take medication: out of stock',
        cooldown: 'Cannot take medication: timing restrictions not met',
        paused: 'Cannot take medication: paused',
        selection_required: 'Choose a location to record this dose.',
        invalid_source: 'Selected location is unavailable for this medication.',
        invalid_amount: 'Invalid dose configured',
        unknown_failure: 'Could not record medication take'
      }

      expected_messages.each do |error, message|
        allow(service).to receive(:call).and_return(TakeMedicationService::Result.new(false, nil, error))

        post api_v1_household_medication_takes_path(household_id),
             params: {
               medication_take: {
                 client_uuid: SecureRandom.uuid,
                 source_type: 'schedule',
                 source_id: schedules(:jane_ibuprofen).portable_id,
                 taken_at: Time.current.iso8601
               }
             },
             headers: api_auth_headers(login_data.fetch('access_token')),
             as: :json

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.dig('error', 'message')).to eq(message)
      end
    end
  end
end
