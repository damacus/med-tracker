# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 error envelope' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'includes request correlation for unauthenticated requests' do
    get api_v1_household_people_path(household_id), as: :json

    expect(response).to have_http_status(:unauthorized)
    expect(response.parsed_body.fetch('error')).to include(
      'code' => 'unauthorized',
      'message' => 'Authentication required',
      'request_id' => response.headers.fetch('X-Request-Id')
    )
  end

  it 'includes request correlation for request validation failures' do
    get api_v1_household_people_path(household_id),
        params: { updated_since: 'not-a-date' },
        headers: headers,
        as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.fetch('error')).to include(
      'code' => 'unprocessable_content',
      'message' => 'updated_since must be ISO8601',
      'request_id' => response.headers.fetch('X-Request-Id')
    )
  end

  it 'keeps validation details under errors while adding request correlation' do
    patch api_v1_household_medication_path(household_id, medications(:paracetamol).id),
          params: { medication: { name: '' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.fetch('error')).to include(
      'code' => 'validation_failed',
      'message' => 'Validation failed',
      'request_id' => response.headers.fetch('X-Request-Id'),
      'errors' => include('name')
    )
  end
end
