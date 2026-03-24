# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 people' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  let(:user) { users(:jane) }

  describe 'GET /api/v1/people' do
    it 'returns only people in the signed-in user scope' do
      login_data = api_login(user)

      get api_v1_people_path, headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:ok)

      returned_ids = response.parsed_body.fetch('data').map { |person| person.fetch('id') }
      expect(returned_ids).to include(people(:jane).id, people(:child_patient).id)
      expect(returned_ids).not_to include(people(:john).id)
    end

    it 'returns a structured error for an invalid updated_since filter' do
      login_data = api_login(user)

      get api_v1_people_path,
          params: { updated_since: 'not-a-timestamp' },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'code')).to eq('unprocessable_content')
    end
  end

  describe 'GET /api/v1/people/:id' do
    it 'returns forbidden for a person outside scope' do
      login_data = api_login(user)

      get api_v1_person_path(people(:john)),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:not_found)
    end
  end
end
