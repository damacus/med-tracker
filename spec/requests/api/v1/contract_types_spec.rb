# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 JSON value types' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'rejects numeric JSON decimal values with the validation envelope' do
    post api_v1_household_medications_path(household_id),
         params: {
           medication: {
             name: 'Typed API Medication', location_id: locations(:home).id,
             current_supply: 10, reorder_threshold: '1'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body).to include(
      'error' => include(
        'code' => 'validation_failed',
        'errors' => include('current_supply' => ['must be a string'])
      )
    )
  end

  it 'rejects numeric JSON resource identifiers with the validation envelope' do
    post api_v1_household_schedules_path(household_id),
         params: {
           schedule: {
             person_id: people(:john).id,
             medication_id: medications(:paracetamol).portable_id,
             dose_amount: '500',
             dose_unit: 'mg',
             start_date: '2026-02-25',
             end_date: '2026-12-31'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'errors')).to include('person_id' => ['must be a string'])
  end

  it 'rejects numeric JSON identifiers in identifier arrays' do
    post api_v1_household_health_events_path(household_id),
         params: {
           health_event: {
             person_id: people(:john).portable_id,
             event_kind: 'illness',
             title: 'Typed API event',
             started_on: '2026-02-25',
             medication_ids: [medications(:paracetamol).id]
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'errors')).to include('medication_ids' => ['must be a string'])
  end
end
