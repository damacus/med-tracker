# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 auth sessions' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:jane) }
  let(:account) { user.person.account }

  describe 'API session household selection, listing, and revocation' do
    it 'lists households, lists sessions, and revokes a selected session' do
      login_data = api_login(user)
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_auth_households_path, headers: headers, as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.fetch('data').first).to include('id', 'name', 'role')

      get api_v1_auth_sessions_path, headers: headers, as: :json

      expect(response).to have_http_status(:ok)
      session_id = response.parsed_body.fetch('data').first.fetch('id')

      delete api_v1_auth_session_path(session_id), headers: headers, as: :json

      expect(response).to have_http_status(:no_content)
      expect(ApiSession.find(session_id)).to be_revoked_at
    end

    it 'rejects expired access tokens on auth session management endpoints' do
      login_data = api_login(user)
      headers = api_auth_headers(login_data.fetch('access_token'))
      api_session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))
      api_session.update!(access_expires_at: 1.minute.ago)

      get api_v1_auth_households_path, headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)

      get api_v1_auth_sessions_path, headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)

      delete api_v1_auth_session_path(api_session.id), headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(api_session.reload.revoked_at).to be_nil
    end

    it 'rejects auth management for a bearer token after the account is locked' do
      login_data = api_login(user)
      headers = api_auth_headers(login_data.fetch('access_token'))
      api_session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))
      last_used_at = api_session.last_used_at
      lock_account!(account)

      get api_v1_auth_sessions_path, headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(api_session.reload.last_used_at).to eq(last_used_at)
    end

    it 'rejects auth management for an unknown bearer token' do
      headers = api_auth_headers('mt_unknown_access_token')

      get api_v1_auth_households_path, headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)

      get api_v1_auth_sessions_path, headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)

      delete api_v1_auth_session_path(ApiSession.maximum(:id).to_i + 1), headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)
    end
  end

  describe 'DELETE /api/v1/auth/logout' do
    before do
      clear_2fa_for_account(account)
      AccountLockout.where(account_id: account.id).delete_all if defined?(AccountLockout)
    end

    it 'revokes the current access token' do
      login_data = api_login(user)
      headers = api_auth_headers(login_data.fetch('access_token'))
      household_id = login_data.dig('household', 'id')

      delete api_v1_auth_logout_path, headers: headers, as: :json

      expect(response).to have_http_status(:no_content)

      get api_v1_household_me_path(household_id), headers: headers, as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('unauthorized')
    end

    it 'treats missing bearer tokens as an idempotent logout' do
      delete api_v1_auth_logout_path, as: :json

      expect(response).to have_http_status(:no_content)
    end

    it 'records a revoked audit event' do
      login_data = api_login(user)
      headers = api_auth_headers(login_data.fetch('access_token'))

      expect do
        delete api_v1_auth_logout_path, headers: headers, as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/revoked').count
      }.by(1)
    end
  end

  def lock_account!(account)
    AccountLockout.create!(
      account_id: account.id,
      key: SecureRandom.hex(16),
      deadline: 30.minutes.from_now
    )
  end
end
