# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 auth sessions' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:jane) }
  let(:account) { user.person.account }
  let(:oidc_issuer) { 'https://issuer.example.test' }
  let(:oidc_client_id) { 'medtracker-mobile-test' }
  let(:oidc_client_secret) { 'oidc-test-secret' }

  before do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('OIDC_ISSUER_URL', nil).and_return(oidc_issuer)
    allow(ENV).to receive(:fetch).with('OIDC_MOBILE_CLIENT_ID', nil).and_return(oidc_client_id)
    allow(ENV).to receive(:fetch).with('OIDC_CLIENT_ID', nil).and_return(oidc_client_id)
    allow(ENV).to receive(:fetch).with('OIDC_CLIENT_SECRET', nil).and_return(oidc_client_secret)
  end

  describe 'POST /api/v1/auth/login' do
    before do
      clear_2fa_for_account(account)
      AccountLockout.where(account_id: account.id).delete_all if defined?(AccountLockout)
    end

    def create_api_household_for(user)
      ensure_api_household_for(user)
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
      expected_membership = household.household_memberships.find_by!(account: account)

      expect(response).to have_http_status(:created)
      expect(api_session.household_membership).to eq(expected_membership)
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
      expect(response.parsed_body.dig('error', 'request_id')).to eq(response.headers.fetch('X-Request-Id'))
    end

    it 'records API password failures and locks the account after five attempts' do
      create_api_household_for(user)

      expect do
        5.times do
          post api_v1_auth_login_path,
               params: { email: user.email_address, password: 'wrong-password' },
               as: :json
        end
      end.to change { AccountLockout.active.where(account_id: account.id).count }.from(0).to(1)

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_credentials')
    end

    it 'rejects an unknown email address without revealing account existence' do
      post api_v1_auth_login_path,
           params: {
             email: 'missing-api-user@example.test',
             password: 'password'
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_credentials')
    end

    it 'clears accumulated API password failures after successful password login' do
      create_api_household_for(user)

      2.times do
        post api_v1_auth_login_path,
             params: { email: user.email_address, password: 'wrong-password' },
             as: :json
      end

      expect(AccountLoginFailure.find_by(account_id: account.id)&.number).to eq(2)

      post api_v1_auth_login_path,
           params: { email: user.email_address, password: 'password' },
           as: :json

      expect(response).to have_http_status(:created)
      expect(AccountLoginFailure.find_by(account_id: account.id)).to be_nil
    end

    it 'rejects a requested household that is not active for the account' do
      create_api_household_for(user)

      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'password',
             household_id: Household.maximum(:id).to_i + 10_000
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.dig('error', 'code')).to eq('invalid_credentials')
    end

    it 'rejects login without an explicit household when the account has multiple active memberships' do
      create_api_household_for(user)
      second_household = create(:household)
      second_household.household_memberships.create!(
        account: account,
        role: :member,
        status: :active
      )

      post api_v1_auth_login_path,
           params: {
             email: user.email_address,
             password: 'password'
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
      invalid_credentials_response = [response.status, response.parsed_body.fetch('error').slice('code', 'message')]

      expect do
        post api_v1_auth_login_path,
             params: {
               email: user.email_address,
               password: 'password'
             },
             as: :json
      end.not_to change(ApiSession, :count)

      expect([response.status, response.parsed_body.fetch('error').slice('code', 'message')])
        .to eq(invalid_credentials_response)
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
      invalid_credentials_response = [response.status, response.parsed_body.fetch('error').slice('code', 'message')]

      expect do
        post api_v1_auth_login_path,
             params: {
               email: user.email_address,
               password: 'password'
             },
             as: :json
      end.not_to change(ApiSession, :count)

      expect([response.status, response.parsed_body.fetch('error').slice('code', 'message')])
        .to eq(invalid_credentials_response)
    end
  end

  describe 'POST /api/v1/auth/oidc_exchange' do
    before do
      AccountLockout.where(account_id: account.id).delete_all if defined?(AccountLockout)
      AccountIdentity.find_or_create_by!(account: account, provider: 'oidc', uid: 'jane-oidc-sub')
      ensure_api_household_for(user)
    end

    it 'exchanges a valid OIDC identity token for an API session' do
      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'nonce-1'),
             nonce: 'nonce-1',
             code_verifier: 'pkce-verifier',
             device_name: 'RSpec Mobile'
           },
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'access_token')).to be_present
      expect(response.parsed_body.dig('data', 'refresh_token')).to be_present
      expect(response.parsed_body.dig('data', 'me', 'email_address')).to eq(user.email_address)
      expect(ApiSession.order(:id).last.device_name).to eq('RSpec Mobile')
    end

    it 'binds OIDC exchange to a requested household membership' do
      household = account.household_memberships.active.first.household

      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'household-nonce'),
             nonce: 'household-nonce',
             code_verifier: 'pkce-verifier',
             household_id: household.id
           },
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'household', 'id')).to eq(household.id)
    end

    it 'records OIDC MFA proof when the identity token includes an MFA authentication method' do
      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'mfa-nonce', amr: %w[pwd otp]),
             nonce: 'mfa-nonce',
             code_verifier: 'pkce-verifier'
           },
           as: :json

      expect(response).to have_http_status(:created)
      expect(ApiSession.order(:id).last).to be_oidc_mfa_verified
    end

    it 'rejects OIDC exchange when required token inputs are missing' do
      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'missing-verifier'),
             nonce: 'missing-verifier'
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)

      post api_v1_auth_oidc_exchange_path,
           params: {
             nonce: 'missing-token',
             code_verifier: 'pkce-verifier'
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
    end

    it 'rejects OIDC exchange with a blank subject' do
      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: '', nonce: 'blank-subject'),
             nonce: 'blank-subject',
             code_verifier: 'pkce-verifier'
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
    end

    it 'rejects linked OIDC accounts that have no active person user' do
      household = create(:household)
      linked_account = Account.create!(email: 'oidc-no-person@example.test', status: :verified)
      AccountIdentity.create!(account: linked_account, provider: 'oidc', uid: 'no-person-sub')
      household.household_memberships.create!(account: linked_account, role: :member, status: :active)

      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'no-person-sub', nonce: 'no-person-nonce'),
             nonce: 'no-person-nonce',
             code_verifier: 'pkce-verifier'
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
    end

    it 'rejects invalid issuer, audience, expiry, nonce, replay, locked account, and revoked membership cases' do
      invalid_cases = [
        [oidc_token(sub: 'jane-oidc-sub', issuer: 'https://evil.example.test', nonce: 'bad-issuer'), 'bad-issuer'],
        [oidc_token(sub: 'jane-oidc-sub', audience: 'wrong-client', nonce: 'bad-audience'), 'bad-audience'],
        [oidc_token(sub: 'jane-oidc-sub', exp: 1.minute.ago.to_i, nonce: 'expired'), 'expired'],
        [oidc_token(sub: 'jane-oidc-sub', nonce: 'expected-nonce'), 'different-nonce']
      ]

      invalid_cases.each do |token, nonce|
        post api_v1_auth_oidc_exchange_path,
             params: { id_token: token, nonce: nonce, code_verifier: 'pkce-verifier' },
             as: :json

        expect(response).to have_http_status(:unauthorized)
        expect(response.parsed_body.dig('error', 'code')).to eq('invalid_oidc_exchange')
      end

      replay_token = oidc_token(sub: 'jane-oidc-sub', nonce: 'replay-nonce')
      post api_v1_auth_oidc_exchange_path,
           params: { id_token: replay_token, nonce: 'replay-nonce', code_verifier: 'pkce-verifier' },
           as: :json
      post api_v1_auth_oidc_exchange_path,
           params: { id_token: replay_token, nonce: 'replay-nonce', code_verifier: 'pkce-verifier' },
           as: :json
      expect(response).to have_http_status(:unauthorized)

      lock_account!(account)
      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'locked-nonce'),
             nonce: 'locked-nonce',
             code_verifier: 'pkce-verifier'
           },
           as: :json
      expect(response).to have_http_status(:unauthorized)
      AccountLockout.where(account_id: account.id).delete_all

      account.household_memberships.active.first.update!(status: :revoked)
      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'revoked-nonce'),
             nonce: 'revoked-nonce',
             code_verifier: 'pkce-verifier'
           },
           as: :json
      expect(response).to have_http_status(:unauthorized)
    end
  end

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

    it 'rejects an unknown refresh token' do
      post api_v1_auth_refresh_path,
           params: { refresh_token: 'mt_unknown_refresh_token' },
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

  def oidc_token(sub:, nonce:, **overrides)
    JWT.encode(
      {
        iss: overrides.fetch(:issuer, oidc_issuer),
        aud: overrides.fetch(:audience, oidc_client_id),
        exp: overrides.fetch(:exp, 15.minutes.from_now.to_i),
        iat: Time.current.to_i,
        sub: sub,
        nonce: nonce,
        amr: overrides[:amr]
      }.compact,
      oidc_client_secret,
      'HS256'
    )
  end
end
