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

    it 'rejects a tampered history cursor' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medication_takes_path(household_id),
          params: { cursor: 'tampered' },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'message')).to eq('cursor is invalid')
    end

    it 'filters history by a policy-visible portable person ID and time range' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      expected_take = medication_takes(:jane_morning_ibuprofen)

      get api_v1_household_medication_takes_path(household_id),
          params: {
            person_id: user.person.portable_id,
            from: (expected_take.taken_at - 1.minute).iso8601,
            to: (expected_take.taken_at + 1.minute).iso8601
          },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      data = response.parsed_body.fetch('data')
      expect(response).to have_http_status(:ok)
      expect(data.pluck('portable_id')).to include(expected_take.portable_id)
      expect(data.pluck('person_portable_id').uniq).to eq([user.person.portable_id])
      expect(response.parsed_body.fetch('meta')).to include('next_cursor' => nil, 'has_more' => false)
      expect(data).to all(include('reversal' => nil))
    end

    it 'rejects hidden and unknown person history filters without leakage' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      membership = HouseholdMembership.find_by!(household_id:, account: user.person.account)
      granted_ids = PersonAccessGrant.active.where(household_membership: membership).select(:person_id)
      hidden_person = Person.where(household_id:).where.not(id: granted_ids).first!
      responses = [hidden_person.portable_id, SecureRandom.uuid].map do |person_id|
        get api_v1_household_medication_takes_path(household_id),
            params: { person_id: person_id },
            headers: api_auth_headers(login_data.fetch('access_token')),
            as: :json
        [response.status, response.parsed_body]
      end

      expect(responses.map(&:first)).to eq([422, 422])
      expect(responses.map { |result| result.last.dig('error', 'message') }.uniq).to eq(['person_id is invalid'])
    end

    it 'rejects invalid history timestamps and inverted ranges' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      filters = [
        [{ from: 'not-a-time' }, 'from must be ISO8601'],
        [{ to: 'not-a-time' }, 'to must be ISO8601'],
        [{ from: '2026-07-18T00:00:00Z', to: '2026-07-17T00:00:00Z' }, 'from must be before or equal to to']
      ]

      filters.each do |params, message|
        get api_v1_household_medication_takes_path(household_id),
            params: params,
            headers: api_auth_headers(login_data.fetch('access_token')),
            as: :json

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.parsed_body.dig('error', 'message')).to eq(message)
      end
    end

    it 'paginates ties without duplicates and clamps cursor pages to 100 rows' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      taken_at = Time.iso8601('2026-01-15T12:00:00Z')
      schedule = schedules(:jane_ibuprofen)
      created_takes = Array.new(103) do
        MedicationTake.create!(
          household: schedule.household,
          schedule: schedule,
          person_medication: nil,
          taken_at: taken_at,
          dose_amount: schedule.dose_amount,
          dose_unit: schedule.dose_unit,
          skip_stock_mutation: true
        )
      end
      filters = {
        person_id: user.person.portable_id,
        from: (taken_at - 1.minute).iso8601,
        to: (taken_at + 1.minute).iso8601,
        per_page: 500
      }

      get api_v1_household_medication_takes_path(household_id),
          params: filters,
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json
      first_page = response.parsed_body
      cursor = first_page.dig('meta', 'next_cursor')

      get api_v1_household_medication_takes_path(household_id),
          params: filters.merge(cursor: cursor),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json
      expect(response).to have_http_status(:ok), response.body
      second_page = response.parsed_body
      returned_ids = (first_page.fetch('data') + second_page.fetch('data')).pluck('portable_id')

      expect(first_page.fetch('data').size).to eq(100)
      expect(first_page.fetch('meta')).to include('per_page' => 100, 'has_more' => true)
      expect(second_page.fetch('data').size).to eq(3)
      expect(second_page.fetch('meta')).to include('next_cursor' => nil, 'has_more' => false)
      expect(returned_ids).to match_array(created_takes.map(&:portable_id))
      expect(returned_ids.uniq.size).to eq(103)
      expect(cursor).not_to include(taken_at.iso8601, created_takes.last.id.to_s)
    end

    it 'binds a cursor to its original filters' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      taken_at = Time.iso8601('2026-01-16T12:00:00Z')
      create_list(
        :medication_take,
        2,
        schedule: schedules(:jane_ibuprofen),
        person_medication: nil,
        taken_at: taken_at,
        skip_stock_mutation: true
      )
      filters = {
        person_id: user.person.portable_id,
        from: (taken_at - 1.minute).iso8601,
        to: (taken_at + 1.minute).iso8601,
        per_page: 1
      }
      get api_v1_household_medication_takes_path(household_id),
          params: filters,
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json
      cursor = response.parsed_body.dig('meta', 'next_cursor')

      get api_v1_household_medication_takes_path(household_id),
          params: filters.merge(cursor: cursor, from: (taken_at - 2.minutes).iso8601),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'message')).to eq('cursor is invalid')
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
      service = instance_double(MedicationAdministration::RecordDose)
      take = create(
        :medication_take,
        schedule: schedules(:jane_ibuprofen),
        person_medication: nil,
        skip_stock_mutation: true
      )
      allow(MedicationAdministration::RecordDose).to receive(:new).and_return(service)
      allow(service).to receive(:call).and_return(MedicationAdministration::RecordDose::Result.new(true, take, nil))

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
      service = instance_double(MedicationAdministration::RecordDose)
      allow(MedicationAdministration::RecordDose).to receive(:new).and_return(service)
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
        allow(service).to receive(:call).and_return(MedicationAdministration::RecordDose::Result.new(false, nil, error))

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
