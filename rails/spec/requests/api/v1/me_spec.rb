# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 me' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }

  describe 'GET /api/v1/households/:household_id/me' do
    it 'authenticates and records session activity as the forced-RLS application role' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')

      get api_v1_household_me_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(user.id)
    end

    it 'rejects revoked membership before changing session activity under row security' do
      login_data = api_login(user)
      session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))
      session.household_membership.update!(status: :revoked)
      previous_activity = session.last_used_at
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')

      get api_v1_household_me_path(login_data.dig('household', 'id')),
          headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(session.reload.last_used_at).to eq(previous_activity)
    end

    it 'returns the signed-in user profile' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_me_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)

      data = response.parsed_body.fetch('data')
      expect(data.fetch('id')).to eq(user.id)
      expect(data.fetch('email_address')).to eq(user.email_address)
      expect(data.fetch('active')).to eq(user.active)
      expect(data).to have_key('membership_role')

      person_data = data.fetch('person')
      expect(person_data.fetch('id')).to eq(user.person.id)

      account_data = data.fetch('account')
      expect(account_data.fetch('id')).to eq(user.person.account.id)
      expect(account_data.fetch('email')).to eq(user.person.account.email)
    end

    it 'returns unauthorized without a valid token' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_me_path(household_id),
          headers: api_auth_headers('invalid_token'),
          as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('unauthorized')
    end
  end
end
