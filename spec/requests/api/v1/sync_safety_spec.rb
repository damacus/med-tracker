# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 sync safety primitives' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'replays matching idempotent mutation responses without storing request bodies' do
    idempotency_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    payload = {
      medication: {
        name: 'API Idempotent Saline',
        location_id: locations(:home).id,
        dose_amount: 5,
        dose_unit: 'ml',
        current_supply: 100,
        reorder_threshold: 10
      }
    }

    expect do
      post api_v1_household_medications_path(household_id), params: payload, headers: idempotency_headers, as: :json
    end.to change(Medication, :count).by(1)

    first_body = response.parsed_body
    stored_key = ApiIdempotencyKey.find_by!(key: idempotency_headers.fetch('Idempotency-Key'))
    expect(stored_key.request_digest).to be_present
    expect(stored_key).not_to respond_to(:request_body)

    expect do
      post api_v1_household_medications_path(household_id), params: payload, headers: idempotency_headers, as: :json
    end.not_to change(Medication, :count)

    expect(response).to have_http_status(:created)
    expect(response.parsed_body).to eq(first_body)
    expect(response.headers['Idempotency-Replayed']).to eq('true')
  end

  it 'rejects reused idempotency keys with a different payload' do
    idempotency_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    payload = {
      medication: {
        name: 'API Idempotent Original',
        location_id: locations(:home).id,
        dose_amount: 5,
        dose_unit: 'ml'
      }
    }

    post api_v1_household_medications_path(household_id), params: payload, headers: idempotency_headers, as: :json

    post api_v1_household_medications_path(household_id),
         params: payload.deep_merge(medication: { name: 'API Idempotent Changed' }),
         headers: idempotency_headers,
         as: :json

    expect(response).to have_http_status(:conflict)
    expect(response.parsed_body.dig('error', 'code')).to eq('idempotency_key_reused')
  end

  it 'finds resources by portable id without leaking cross-household records' do
    medication = medications(:paracetamol)
    other_household = Household.create!(name: 'Other Portable Household', slug: 'other-portable-household')
    other_location = create(:location, household: other_household, name: 'Other Portable Location')
    other_medication = create(:medication, household: other_household, location: other_location, name: 'Other Portable')

    get api_v1_household_medication_path(household_id, medication.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(medication.id)

    get api_v1_household_medication_path(household_id, other_medication.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:not_found)
  end

  it 'finds resources by future UUID version portable ids' do
    portable_id = '01890f13-7d4a-7cc5-98f3-90d8d3934b8e'
    medication = create(:medication, household: Household.find(household_id), portable_id: portable_id)

    get api_v1_household_medication_path(household_id, medication.portable_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'portable_id')).to eq(portable_id)
  end

  it 'uses etags and if-match to reject stale writes' do
    medication = medications(:paracetamol)

    get api_v1_household_medication_path(household_id, medication.id), headers: headers, as: :json

    expect(response.headers['ETag']).to be_present
    current_etag = response.headers.fetch('ETag')

    patch api_v1_household_medication_path(household_id, medication.id),
          params: { medication: { current_supply: 42 } },
          headers: headers.merge('If-Match' => '"stale-etag"'),
          as: :json

    expect(response).to have_http_status(:conflict)
    expect(response.parsed_body.dig('error', 'code')).to eq('conflict')

    patch api_v1_household_medication_path(household_id, medication.id),
          params: { medication: { current_supply: 42 } },
          headers: headers.merge('If-Match' => current_etag),
          as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'current_supply')).to eq('42.0')
  end

  it 'records redacted API change metadata for successful writes' do
    medication = medications(:paracetamol)

    expect do
      patch api_v1_household_medication_path(household_id, medication.id),
            params: { medication: { name: 'Sensitive API Medicine', current_supply: 88 } },
            headers: headers,
            as: :json
    end.to change(ApiChangeEvent, :count).by(1)

    event = ApiChangeEvent.order(:created_at).last
    expect(event).to have_attributes(
      household_id: household_id,
      record_type: 'Medication',
      record_id: medication.id,
      action: 'update'
    )
    expect(event.metadata.to_json).not_to include('Sensitive API Medicine')
  end
end
