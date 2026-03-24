# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 resources' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules,
           :person_medications, :medication_takes, :carer_relationships

  let(:user) { users(:admin) }

  before do
    people(:admin).create_notification_preference!(
      enabled: true,
      morning_time: '08:00',
      afternoon_time: '14:00',
      evening_time: '18:00',
      night_time: '22:00'
    )
  end

  it 'returns the current user profile' do
    login_data = api_login(user)

    get api_v1_me_path, headers: api_auth_headers(login_data.fetch('access_token')), as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'email_address')).to eq(user.email_address)
    expect(response.parsed_body.dig('data', 'account', 'status')).to eq('verified')
  end

  it 'returns the core read-only collections' do
    login_data = api_login(user)
    headers = api_auth_headers(login_data.fetch('access_token'))

    {
      api_v1_locations_path => locations(:home).id,
      api_v1_medications_path => medications(:paracetamol).id,
      api_v1_schedules_path => schedules(:john_paracetamol).id,
      api_v1_person_medications_path => person_medications(:john_vitamin_d).id,
      api_v1_medication_takes_path => medication_takes(:john_morning_paracetamol).id
    }.each do |path, expected_id|
      get path, headers: headers, as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.fetch('data').map { |row| row.fetch('id') }).to include(expected_id)
      expect(response.parsed_body.fetch('meta')).to include('page' => 1)
    end
  end

  it 'returns the signed-in users notification preference' do
    login_data = api_login(user)

    get api_v1_notification_preference_path,
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'person_id')).to eq(people(:admin).id)
    expect(response.parsed_body.dig('data', 'morning_time')).to eq('08:00:00')
  end
end
