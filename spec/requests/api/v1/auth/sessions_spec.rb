# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 auth sessions' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:jane) }
  let(:account) { user.person.account }

  describe 'POST /api/v1/auth/login' do
    before do
      clear_2fa_for_account(account)
      AccountLockout.where(account_id: account.id).delete_all if defined?(AccountLockout)
    end

    def create_api_household_for(user)
      household = user.person.household
      membership = household.household_memberships.find_or_initialize_by(account: user.person.account)
      membership.update!(
        person: user.person,
        role: :owner,
        status: :active
      )
      household
    end

    it 'returns access and refresh tokens with the current user payload' do
      create_api_household_for(user)

      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'password',
             device_name: 'RSpec iPhone'
           },
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'access_token')).to be_present
      expect(response.parsed_body.dig('data', 'refresh_token')).to be_present
      expect(response.parsed_body.dig('data', 'me', 'email_address')).to eq(user.email_address)

      api_session = ApiSession.order(:id).last
      expect(api_session.account).to eq(user.person.account)
      expect(api_session.device_name).to eq('RSpec iPhone')
    end

    it 'binds login to the sole active household membership when household_id is omitted' do
      household = create_api_household_for(user)

      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'password',
             device_name: 'RSpec iPhone'
           },
           as: :json

      api_session = ApiSession.order(:id).last
      expect(response).to have_http_status(:created)
      expect(api_session.household_membership).to eq(household.household_memberships.find_by!(account: account))
      expect(response.parsed_body.dig('data', 'household', 'id')).to eq(household.id)
    end

    it 'binds sessions to the requested active household membership' do
      household = people(:jane).household
      membership = household.household_memberships.find_or_initialize_by(account: user.person.account)
      membership.update!(
        person: people(:jane),
        role: :owner,
        status: :active
      )

      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'password',
             household_id: household.id,
             device_name: 'RSpec iPhone'
           },
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'household', 'id')).to eq(household.id)

      api_session = ApiSession.order(:id).last
      expect(api_session.household_membership).to eq(membership)
      expect(api_session.permissions_version).to eq(membership.permissions_version)
    end

    it 'records a redacted session creation audit event' do
      create_api_household_for(user)

      expect do
        post api_v1_auth_login_path,
             params: {
               email: user.email_address,
               password: 'password',
               device_name: 'RSpec iPhone'
             },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/created').count
      }.by(1)

      access_token = response.parsed_body.dig('data', 'access_token')
      refresh_token = response.parsed_body.dig('data', 'refresh_token')
      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      data = JSON.parse(version.object)

      expect(version.item_id).to eq(user.person.account.id)
      expect(version.whodunnit).to eq(user.id.to_s)
      expect(data['device_name_present']).to be true
      expect(version.object).not_to include(access_token)
      expect(version.object).not_to include(refresh_token)
    end

    it 'rejects an invalid password' do
      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'wrong-password'
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_credentials')
    end

    it 'rejects a locked account without creating an API session' do
      create_api_household_for(user)
      lock_account!(account)

      expect do
        post api_v1_auth_login_path,
             params: {
               email: user.email_address,
               password: 'password'
             },
             as: :json
      end.not_to change(ApiSession, :count)

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_credentials')
    end

    it 'returns the generic invalid credentials response when TOTP is configured' do
      create_api_household_for(user)
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'wrong-password'
           },
           as: :json
      invalid_credentials_response = [response.status, response.parsed_body]

      expect do
        post api_v1_auth_login_path,
             params: {
               email: user.email_address,
               password: 'password'
             },
             as: :json
      end.not_to change(ApiSession, :count)

      expect([response.status, response.parsed_body]).to eq(invalid_credentials_response)
    end

    it 'returns the generic invalid credentials response when WebAuthn is configured' do
      create_api_household_for(user)
      account.account_webauthn_keys.create!(
        webauthn_id: 'api-login-passkey',
        public_key: 'api-login-public-key',
        sign_count: 0,
        nickname: 'API Login Passkey'
      )

      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'wrong-password'
           },
           as: :json
      invalid_credentials_response = [response.status, response.parsed_body]

      expect do
        post api_v1_auth_login_path,
             params: {
               email: user.email_address,
               password: 'password'
             },
             as: :json
      end.not_to change(ApiSession, :count)

      expect([response.status, response.parsed_body]).to eq(invalid_credentials_response)
    end
  end

  describe 'POST /api/v1/auth/refresh' do
    before do
      clear_2fa_for_account(account)
      AccountLockout.where(account_id: account.id).delete_all if defined?(AccountLockout)
    end

    it 'rotates refresh tokens and invalidates the old refresh token' do
      login_data = api_login(user)

      post api_v1_auth_refresh_path,
           params: { refresh_token: login_data.fetch('refresh_token') },
           as: :json

      expect(response).to have_http_status(:ok)
      new_refresh_token = response.parsed_body.dig('data', 'refresh_token')
      expect(new_refresh_token).to be_present
      expect(new_refresh_token).not_to eq(login_data.fetch('refresh_token'))

      post api_v1_auth_refresh_path,
           params: { refresh_token: login_data.fetch('refresh_token') },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_refresh_token')
    end

    it 'records a redacted rotation audit event' do
      login_data = api_login(user)

      expect do
        post api_v1_auth_refresh_path,
             params: { refresh_token: login_data.fetch('refresh_token') },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/rotated').count
      }.by(1)

      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      expect(version.item_id).to eq(user.person.account.id)
      expect(version.object).not_to include(response.parsed_body.dig('data', 'access_token'))
      expect(version.object).not_to include(response.parsed_body.dig('data', 'refresh_token'))
    end

    it 'records an expired audit event for a known expired refresh token' do
      login_data = api_login(user)
      session = ApiSession.lookup_by_refresh_token(login_data.fetch('refresh_token'))
      session.update!(refresh_expires_at: 1.minute.ago)

      expect do
        post api_v1_auth_refresh_path,
             params: { refresh_token: login_data.fetch('refresh_token') },
             as: :json
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/expired').count
      }.by(1)

      expect(response).to have_http_status(:unauthorized)
    end

    it 'rejects refresh tokens while the account is locked' do
      login_data = api_login(user)
      lock_account!(account)

      post api_v1_auth_refresh_path,
           params: { refresh_token: login_data.fetch('refresh_token') },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_refresh_token')
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
