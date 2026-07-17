# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 auth sessions' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:jane) }
  let(:account) { user.person.account }

  before do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('OIDC_ISSUER_URL', nil).and_return(oidc_issuer)
    allow(ENV).to receive(:fetch).with('OIDC_DISCOVERY_URL', nil).and_return(oidc_discovery_url)
    allow(ENV).to receive(:fetch).with('OIDC_MOBILE_CLIENT_ID', nil).and_return(oidc_client_id)
    allow(ENV).to receive(:fetch).with('OIDC_MOBILE_REDIRECT_URIS', nil).and_return(oidc_redirect_uri)
    allow(ENV).to receive(:fetch).with('OIDC_ALLOWED_ENDPOINT_ORIGINS', nil).and_return(oidc_issuer)
    allow(Addrinfo).to receive(:getaddrinfo).and_return([Addrinfo.tcp('8.8.8.8', 443)])
    stub_oidc_discovery
    stub_oidc_jwks
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

    it 'exchanges an authorization code with PKCE for an API session without a client secret' do
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'nonce-1'))

      exchange_oidc(nonce: 'nonce-1', device_name: 'RSpec Mobile')

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'access_token')).to be_present
      expect(response.parsed_body.dig('data', 'refresh_token')).to be_present
      expect(response.parsed_body.dig('data', 'me', 'email_address')).to eq(user.email_address)
      expect(ApiSession.order(:id).last.device_name).to eq('RSpec Mobile')
      expect(WebMock).to have_requested(:post, oidc_token_endpoint).with(
        body: hash_including(
          'code' => 'authorization-code',
          'code_verifier' => oidc_code_verifier,
          'client_id' => oidc_client_id,
          'redirect_uri' => oidc_redirect_uri,
          'grant_type' => 'authorization_code'
        )
      )
      expect(WebMock).not_to have_requested(:post, oidc_token_endpoint).with(body: /client_secret/)
    end

    it 'binds OIDC exchange to the sole active operational household membership' do
      household = account.household_memberships.active.sole.household
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'household-nonce'))

      exchange_oidc(nonce: 'household-nonce')

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'household', 'id')).to eq(household.id)
    end

    it 'records OIDC MFA proof when the identity token includes an MFA authentication method' do
      stub_oidc_token_response(
        id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'mfa-nonce', amr: %w[pwd otp])
      )

      exchange_oidc(nonce: 'mfa-nonce')

      expect(response).to have_http_status(:created)
      expect(ApiSession.order(:id).last).to be_oidc_mfa_verified
    end

    it 'rejects the legacy raw id_token contract and missing required code inputs' do
      post api_v1_auth_oidc_exchange_path,
           params: {
             id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'legacy-token'),
             nonce: 'legacy-token',
             code_verifier: oidc_code_verifier,
             redirect_uri: oidc_redirect_uri
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(WebMock).not_to have_requested(:post, oidc_token_endpoint)

      post api_v1_auth_oidc_exchange_path,
           params: {
             authorization_code: 'authorization-code',
             nonce: 'missing-verifier',
             redirect_uri: oidc_redirect_uri
           },
           as: :json

      expect(response).to have_http_status(:unauthorized)
    end

    it 'returns a generic failure for a rejected PKCE exchange or malformed token response' do
      stub_request(:post, oidc_token_endpoint).to_return(status: 400, body: { error: 'invalid_grant' }.to_json)

      exchange_oidc(nonce: 'bad-pkce')

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body.fetch('error')).to include(
        'code' => 'invalid_oidc_exchange',
        'message' => 'OIDC exchange is invalid'
      )

      stub_request(:post, oidc_token_endpoint).to_return(status: 200, body: { access_token: 'secret' }.to_json)

      exchange_oidc(nonce: 'missing-id-token')

      expect(response).to have_http_status(:unauthorized)
    end

    it 'returns a generic failure for non-object provider responses' do
      expect_non_object_token_responses_rejected
      expect_non_object_discovery_responses_rejected
      expect_non_object_jwks_responses_rejected

      Rails.cache.clear
      stub_oidc_discovery
      stub_request(:get, oidc_jwks_uri).to_return(status: 200, body: { keys: [nil] }.to_json)
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'invalid-jwk-shape'))
      exchange_oidc(nonce: 'invalid-jwk-shape')
      expect_generic_oidc_failure
    end

    it 'returns a generic failure for correctly signed non-object claims sets' do
      [[], nil, 'invalid'].each_with_index do |payload, index|
        stub_oidc_token_response(id_token: signed_oidc_payload(payload))
        exchange_oidc(nonce: "claims-shape-#{index}")

        expect_generic_oidc_failure
      end
    end

    it 'rejects discovery failures and issuer mismatches' do
      stub_request(:get, oidc_discovery_url).to_return(status: 503, body: '')
      exchange_oidc(nonce: 'discovery-failure')
      expect(response).to have_http_status(:unauthorized)

      stub_oidc_discovery(issuer: 'https://other-issuer.example.test')
      exchange_oidc(nonce: 'issuer-mismatch')

      expect(response).to have_http_status(:unauthorized)
    end

    it 'rejects JWKS failures and invalid signatures' do
      stub_request(:get, oidc_jwks_uri).to_return(status: 503, body: '')
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'jwks-failure'))
      exchange_oidc(nonce: 'jwks-failure')
      expect(response).to have_http_status(:unauthorized)

      other_key = OpenSSL::PKey::RSA.generate(2048)
      stub_oidc_token_response(
        id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'bad-signature', signing_key: other_key)
      )
      exchange_oidc(nonce: 'bad-signature')

      expect(response).to have_http_status(:unauthorized)
    end

    it 'rejects invalid issuer, audience, expiry, issued-at, nonce, and subject claims' do
      invalid_cases = {
        'bad-issuer' => { issuer: 'https://evil.example.test' },
        'bad-audience' => { audience: 'wrong-client' },
        'expired' => { exp: 1.minute.ago.to_i },
        'future-issued-at' => { iat: 5.minutes.from_now.to_i },
        'different-nonce' => { token_nonce: 'expected-nonce' },
        'blank-subject' => { sub: '' }
      }

      invalid_cases.each do |nonce, overrides|
        token_nonce = overrides.delete(:token_nonce) || nonce
        subject = overrides.delete(:sub) { 'jane-oidc-sub' }
        stub_oidc_token_response(id_token: oidc_token(sub: subject, nonce: token_nonce, **overrides))
        exchange_oidc(nonce: nonce)

        expect(response).to have_http_status(:unauthorized)
        expect(response.parsed_body.dig('error', 'code')).to eq('invalid_oidc_exchange')
      end
    end

    it 'rejects multiple audiences without a matching authorized party' do
      invalid_claims = [
        { audience: [oidc_client_id, 'other-client'] },
        { audience: [oidc_client_id, 'other-client'], azp: 'wrong-client' },
        { audience: [oidc_client_id, oidc_client_id], azp: oidc_client_id },
        { audience: [oidc_client_id, 7], azp: oidc_client_id }
      ]

      invalid_claims.each_with_index do |claims, index|
        nonce = "multiple-audiences-#{index}"
        stub_oidc_token_response(
          id_token: oidc_token(
            sub: 'jane-oidc-sub',
            nonce: nonce,
            **claims
          )
        )
        exchange_oidc(nonce: nonce)

        expect(response).to have_http_status(:unauthorized)
      end

      nonce = 'multiple-audiences-valid'
      stub_oidc_token_response(
        id_token: oidc_token(
          sub: 'jane-oidc-sub',
          nonce: nonce,
          audience: [oidc_client_id, 'other-client'],
          azp: oidc_client_id
        )
      )
      exchange_oidc(nonce: nonce)

      expect(response).to have_http_status(:created)
    end

    it 'rejects nonce replay and an unlinked identity' do
      replay_token = oidc_token(sub: 'jane-oidc-sub', nonce: 'replay-nonce')
      stub_oidc_token_response(id_token: replay_token)
      exchange_oidc(nonce: 'replay-nonce')
      stub_oidc_token_response(id_token: replay_token)
      exchange_oidc(nonce: 'replay-nonce')

      expect(response).to have_http_status(:unauthorized)

      stub_oidc_token_response(id_token: oidc_token(sub: 'missing-subject', nonce: 'unlinked'))
      exchange_oidc(nonce: 'unlinked')

      expect(response).to have_http_status(:unauthorized)
    end

    it 'rejects locked and deactivated linked accounts' do
      lock_account!(account)
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'locked-nonce'))
      exchange_oidc(nonce: 'locked-nonce')
      expect(response).to have_http_status(:unauthorized)
      AccountLockout.where(account_id: account.id).delete_all

      user.update!(active: false)
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'deactivated-nonce'))
      exchange_oidc(nonce: 'deactivated-nonce')
      expect(response).to have_http_status(:unauthorized)
    end

    it 'returns a one-time selection grant for multiple memberships and exchanges it for a session' do
      second_household = create(:household)
      second_membership = second_household.household_memberships.create!(
        account: account,
        role: :member,
        status: :active
      )
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: 'choose-household'))

      expect { exchange_oidc(nonce: 'choose-household', device_name: 'Android') }
        .not_to change(ApiSession, :count)

      expect(response).to have_http_status(:accepted)
      data = response.parsed_body.fetch('data')
      expect(data).to include(
        'status' => 'household_selection_required',
        'selection_token' => be_present,
        'selection_expires_at' => be_present,
        'households' => contain_exactly(
          include('id' => account.household_memberships.active.first.household_id),
          include('id' => second_household.id)
        )
      )
      grant = ApiHouseholdSelectionGrant.order(:id).last
      expect(grant.token_digest).to eq(ApiHouseholdSelectionGrant.digest(data.fetch('selection_token')))
      expect(grant.token_digest).not_to eq(data.fetch('selection_token'))

      post api_v1_auth_select_household_path,
           params: { selection_token: data.fetch('selection_token'), household_id: second_household.id },
           as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'household', 'id')).to eq(second_household.id)
      expect(grant.reload.used_at).to be_present
      expect(ApiSession.order(:id).last).to have_attributes(
        account: account,
        household_membership: second_membership,
        device_name: 'Android'
      )
    end

    it 'bootstraps and audits a sole household and refresh under the forced-RLS application role' do
      membership = account.household_memberships.active.sole

      with_runtime_role do
        nonce = 'runtime-sole'
        stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: nonce))
        exchange_oidc(nonce: nonce)

        expect(response).to have_http_status(:created)
        expect_runtime_auth_audit('created', membership)
        refresh_token = response.parsed_body.dig('data', 'refresh_token')

        post api_v1_auth_refresh_path, params: { refresh_token: refresh_token }, as: :json

        expect(response).to have_http_status(:ok)
        expect_runtime_auth_audit('rotated', membership)
      end
    end

    it 'selects and audits one of multiple households under the forced-RLS application role' do
      second_household = create(:household)
      second_membership = second_household.household_memberships.create!(
        account: account,
        role: :member,
        status: :active
      )

      with_runtime_role do
        nonce = 'runtime-selection'
        stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: nonce))
        exchange_oidc(nonce: nonce)

        expect(response).to have_http_status(:accepted)
        post api_v1_auth_select_household_path,
             params: {
               selection_token: response.parsed_body.dig('data', 'selection_token'),
               household_id: second_household.id
             },
             as: :json

        expect(response).to have_http_status(:created)
        expect_runtime_auth_audit('created', second_membership)
      end
    end

    it 'returns a generic failure for expired, used, and wrong-account selection grants' do
      household_id = account.household_memberships.active.sole.household_id
      expired_grant, expired_token = ApiHouseholdSelectionGrant.issue_for(account: account)
      expired_grant.update!(expires_at: 1.minute.ago)
      used_grant, used_token = ApiHouseholdSelectionGrant.issue_for(account: account)
      used_grant.update!(used_at: Time.current)
      _, wrong_account_token = ApiHouseholdSelectionGrant.issue_for(account: account)
      other_household = create(:household)

      [
        [expired_token, household_id],
        [used_token, household_id],
        [wrong_account_token, other_household.id]
      ].each do |selection_token, selected_household_id|
        post api_v1_auth_select_household_path,
             params: { selection_token: selection_token, household_id: selected_household_id },
             as: :json

        expect(response).to have_http_status(:unauthorized)
        expect(response.parsed_body.fetch('error')).to include(
          'code' => 'invalid_household_selection',
          'message' => 'Household selection is invalid or expired'
        )
      end
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

  def oidc_code_verifier = 'a' * 64
  def oidc_issuer = 'https://issuer.example.test'
  def oidc_discovery_url = "#{oidc_issuer}/.well-known/openid-configuration"
  def oidc_token_endpoint = "#{oidc_issuer}/oauth/token"
  def oidc_jwks_uri = "#{oidc_issuer}/oauth/keys"
  def oidc_client_id = 'medtracker-mobile-test'
  def oidc_redirect_uri = 'https://mobile.example.test/oauth/callback'
  def oidc_signing_key = @oidc_signing_key ||= OpenSSL::PKey::RSA.generate(2048)
  def oidc_jwk = JWT::JWK.new(oidc_signing_key).export

  def exchange_oidc(nonce:, **params)
    post api_v1_auth_oidc_exchange_path,
         params: {
           authorization_code: 'authorization-code',
           code_verifier: oidc_code_verifier,
           redirect_uri: oidc_redirect_uri,
           nonce: nonce
         }.merge(params),
         as: :json
  end

  def stub_oidc_discovery(issuer: oidc_issuer)
    stub_request(:get, oidc_discovery_url).to_return(
      status: 200,
      headers: { 'Content-Type' => 'application/json' },
      body: {
        issuer: issuer,
        token_endpoint: oidc_token_endpoint,
        jwks_uri: oidc_jwks_uri,
        id_token_signing_alg_values_supported: ['RS256']
      }.to_json
    )
  end

  def stub_oidc_jwks
    stub_request(:get, oidc_jwks_uri).to_return(
      status: 200,
      headers: { 'Content-Type' => 'application/json' },
      body: { keys: [oidc_jwk] }.to_json
    )
  end

  def stub_oidc_token_response(id_token:)
    stub_request(:post, oidc_token_endpoint).to_return(
      status: 200,
      headers: { 'Content-Type' => 'application/json' },
      body: { id_token: id_token, access_token: 'upstream-secret', token_type: 'Bearer' }.to_json
    )
  end

  def oidc_token(sub:, nonce:, signing_key: oidc_signing_key, **overrides)
    signing_jwk = JWT::JWK.new(signing_key)
    JWT.encode(
      oidc_claims(sub, nonce, overrides),
      signing_key,
      'RS256',
      kid: signing_jwk.kid
    )
  end

  def oidc_claims(sub, nonce, overrides)
    {
      iss: overrides.fetch(:issuer, oidc_issuer),
      aud: overrides.fetch(:audience, oidc_client_id),
      exp: overrides.fetch(:exp, 15.minutes.from_now.to_i),
      iat: overrides.fetch(:iat, Time.current.to_i),
      sub: sub,
      nonce: nonce,
      amr: overrides[:amr],
      azp: overrides[:azp]
    }.compact
  end

  def expect_non_object_token_responses_rejected
    [[], nil, 'invalid'].each_with_index do |payload, index|
      stub_request(:post, oidc_token_endpoint).to_return(status: 200, body: payload.to_json)
      exchange_oidc(nonce: "token-shape-#{index}")
      expect_generic_oidc_failure
    end
  end

  def expect_non_object_discovery_responses_rejected
    [[], nil, 'invalid'].each_with_index do |payload, index|
      Rails.cache.clear
      stub_request(:get, oidc_discovery_url).to_return(status: 200, body: payload.to_json)
      exchange_oidc(nonce: "discovery-shape-#{index}")
      expect_generic_oidc_failure
    end
  end

  def expect_non_object_jwks_responses_rejected
    [[], nil, 'invalid'].each_with_index do |payload, index|
      Rails.cache.clear
      stub_oidc_discovery
      stub_request(:get, oidc_jwks_uri).to_return(status: 200, body: payload.to_json)
      stub_oidc_token_response(id_token: oidc_token(sub: 'jane-oidc-sub', nonce: "jwks-shape-#{index}"))
      exchange_oidc(nonce: "jwks-shape-#{index}")
      expect_generic_oidc_failure
    end
  end

  def signed_oidc_payload(payload)
    jwk = JWT::JWK.new(oidc_signing_key)
    JWT.encode(payload, oidc_signing_key, 'RS256', kid: jwk.kid)
  end

  def expect_generic_oidc_failure
    expect(response).to have_http_status(:unauthorized)
    expect(response.parsed_body.fetch('error')).to include(
      'code' => 'invalid_oidc_exchange',
      'message' => 'OIDC exchange is invalid'
    )
  end

  def expect_runtime_auth_audit(action, membership)
    TenantContext.with(
      account: account,
      household: membership.household,
      membership: membership,
      request_id: response.headers.fetch('X-Request-Id')
    ) do
      event = SecurityAuditEvent.find_by!(
        event_type: "auth_token/api_session/#{action}",
        request_id: response.headers.fetch('X-Request-Id')
      )
      expect(event).to have_attributes(
        household_id: membership.household_id,
        actor_membership_id: membership.id
      )
    end
  end

  def with_runtime_role
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      yield
      raise ActiveRecord::Rollback
    end
  end
end
