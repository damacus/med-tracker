# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile Rodauth authorization' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }
  let(:verifier) { 'mobile-code-verifier-with-more-than-forty-three-characters' }
  let!(:client) do
    OauthApplication.create!(name: 'MedTracker Android', client_id: 'mobile-android', client_kind: :mobile,
                             redirect_uri: 'io.damacus.medtracker:/oauth2redirect',
                             scopes: 'medtracker offline_access', token_endpoint_auth_method: 'none')
  end
  let(:authorization_params) do
    { response_type: 'code', response_mode: 'query', client_id: client.client_id,
      redirect_uri: client.redirect_uri, scope: client.scopes, state: 'mobile-state',
      code_challenge: Base64.urlsafe_encode64(Digest::SHA256.digest(verifier), padding: false),
      code_challenge_method: 'S256' }
  end

  it 'publishes OAuth authorization server discovery without signing in' do
    get '/.well-known/oauth-authorization-server'

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body).to include('authorization_endpoint', 'token_endpoint', 'revocation_endpoint')
    expect(response.parsed_body.fetch('code_challenge_methods_supported')).to include('S256')
  end

  it 'publishes registered public mobile client settings without secrets' do
    get '/api/v1/capabilities', as: :json

    oauth = response.parsed_body.dig('data', 'authentication', 'mobile_oauth')
    expect(oauth).to include('discovery_url' => 'http://www.example.com/.well-known/oauth-authorization-server',
                             'household_binding' => 'account', 'inactivity_timeout_days' => 30)
    expect(oauth.fetch('clients')).to include(
      'client_id' => client.client_id, 'name' => client.name,
      'redirect_uris' => [client.redirect_uri], 'scopes' => %w[medtracker offline_access]
    )
    expect(response.body).not_to include('client_secret')
  end

  it 'issues account credentials through Rodauth and the registered native callback' do
    sign_in(user)
    code = authorization_code
    redeem(code)

    expect(response).to have_http_status(:ok), response.body
    expect(response.parsed_body).to include('access_token', 'refresh_token')
    grant = OauthGrant.last
    expect(grant).to have_attributes(client_kind: 'mobile', account: user.person.account,
                                     household_membership_id: nil, person_id: nil, permissions_version: nil)
  end

  it 'resumes mobile authorization after local browser login' do
    ensure_api_household_for(user)
    get '/authorize', params: authorization_params
    expect(response).to redirect_to('/login')

    post '/login', params: { email: user.person.account.email, password: 'password' }
    expect(URI(response.location).path).to eq('/authorize')
    follow_redirect!
    expect(response).to have_http_status(:ok)
    redeem(authorization_code)
    expect(response).to have_http_status(:ok)
  end

  it 'requires an enrolled second factor before issuing a mobile authorization code' do
    ensure_api_household_for(user)
    secret = 'jbswy3dpehpk3pxp'
    visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
    AccountOtpKey.create!(id: user.person.account.id, key: secret, last_use: 5.minutes.ago)
    get '/authorize', params: authorization_params
    post '/login', params: { email: user.person.account.email, password: 'password' }

    post '/authorize', params: authorization_params.merge(scope: client.scopes.split)
    expect(response).to have_http_status(:redirect)
    expect(response.location).not_to start_with(client.redirect_uri)
    expect(OauthGrant.mobile.where(account: user.person.account)).to be_empty

    post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
    redeem(authorization_code)
    expect(response).to have_http_status(:ok)
  end

  it 'rejects code redemption with the wrong verifier' do
    sign_in(user)
    code = authorization_code
    redeem(code, code_verifier: 'another-mobile-verifier-with-more-than-forty-three-characters')

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
  end

  it 'rejects a spent authorization code' do
    sign_in(user)
    code = authorization_code
    redeem(code)
    expect(response).to have_http_status(:ok)
    redeem(code)

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
  end

  it 'uses the same token for two households and applies each current role' do
    sign_in(user)
    redeem(authorization_code)
    headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }
    first = Household.create!(name: 'Mobile first', slug: 'mobile-first')
    second = Household.create!(name: 'Mobile second', slug: 'mobile-second')
    first.household_memberships.create!(account: user.person.account, role: :owner, status: :active)
    second_membership = second.household_memberships.create!(account: user.person.account, role: :owner,
                                                             status: :active)

    [first, second].each do |household|
      get "/api/v1/households/#{household.id}/admin/settings", headers: headers, as: :json
      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'name')).to eq(household.name)
    end

    second.household_memberships.create!(account: accounts(:john_doe), role: :owner, status: :active)
    second_membership.update!(role: :member)
    get "/api/v1/households/#{second.id}/admin/settings", headers: headers, as: :json
    expect(response).to have_http_status(:forbidden)
    get "/api/v1/households/#{first.id}/admin/settings", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
  end

  it 'rejects refresh after the configured inactivity window' do
    sign_in(user)
    redeem(authorization_code)
    refresh_token = response.parsed_body.fetch('refresh_token')
    OauthGrant.last.update!(last_used_at: 31.days.ago)

    post '/token', params: { grant_type: 'refresh_token', client_id: client.client_id,
                             refresh_token: refresh_token }, as: :json

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
  end

  it 'lists households and independently revokes one device session' do
    sign_in(user)
    redeem(authorization_code)
    first_grant = OauthGrant.last
    first_headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }
    redeem(authorization_code)
    second_headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }

    get '/api/v1/auth/households', headers: first_headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('id')).to include(user.person.household_id)

    get '/api/v1/auth/sessions', headers: second_headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('id')).to include(first_grant.id)

    delete "/api/v1/auth/sessions/#{first_grant.id}", headers: second_headers, as: :json
    expect(response).to have_http_status(:no_content)
    get '/api/v1/auth/households', headers: first_headers, as: :json
    expect(response).to have_http_status(:unauthorized)
    get '/api/v1/auth/households', headers: second_headers, as: :json
    expect(response).to have_http_status(:ok)
  end

  it 'keeps an account login when no households are currently operational' do
    sign_in(user)
    user.person.household.update!(lifecycle_state: :held)
    redeem(authorization_code)
    expect(response).to have_http_status(:ok)
    headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }

    get '/api/v1/auth/households', headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to be_empty
    get '/api/v1/auth/sessions', headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').size).to eq(1)
  end

  it 'rotates refresh tokens without extending activity or changing authentication time' do
    sign_in(user)
    redeem(authorization_code)
    old_refresh = response.parsed_body.fetch('refresh_token')
    grant = OauthGrant.last
    grant.update!(last_used_at: 2.days.ago)
    activity = grant.last_used_at
    authenticated_at = grant.authenticated_at

    post '/token', params: { grant_type: 'refresh_token', client_id: client.client_id,
                             refresh_token: old_refresh }, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('refresh_token')).not_to eq(old_refresh)
    expect(grant.reload).to have_attributes(last_used_at: activity, authenticated_at: authenticated_at)
    post '/token', params: { grant_type: 'refresh_token', client_id: client.client_id,
                             refresh_token: old_refresh }, as: :json
    expect(response).to have_http_status(:bad_request)
  end

  it 'rejects missing verifiers and mismatched callbacks without consuming a valid code' do
    sign_in(user)
    code = authorization_code
    redeem(code, code_verifier: nil)
    expect(response).to have_http_status(:bad_request)
    redeem(code, redirect_uri: 'io.damacus.medtracker:/other')
    expect(response).to have_http_status(:bad_request)
    redeem(code)
    expect(response).to have_http_status(:ok)
  end

  it 'uses an account grant to resend invitations in the currently authorised household' do
    sign_in(user)
    redeem(authorization_code)
    headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }
    membership = user.person.account.household_memberships.active.sole
    membership.update!(role: :owner)
    invitation = membership.household.household_invitations.create!(
      email: 'mobile-invite@example.test', membership_role: :member, invited_by_membership: membership
    )

    post "/api/v1/households/#{membership.household_id}/admin/invitations/#{invitation.id}/resend",
         headers: headers, as: :json

    expect(response).to have_http_status(:ok), response.body
  end

  it 'accepts an invitation with an account credential and uses the new household immediately' do
    sign_in(user)
    redeem(authorization_code)
    headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }
    household = Household.create!(name: 'Mobile invitation', slug: 'mobile-invitation')
    inviter = household.household_memberships.create!(account: accounts(:john_doe), role: :owner, status: :active)
    invitation = household.household_invitations.create!(email: user.person.account.email,
                                                         invited_by_membership: inviter, membership_role: :member)

    post '/api/v1/invitations/accept', params: { token: invitation.plain_token }, headers: headers, as: :json
    expect(response).to have_http_status(:ok), response.body
    get "/api/v1/households/#{household.id}/profile", headers: headers, as: :json
    expect(response).to have_http_status(:ok), response.body
  end

  def authorization_code
    post '/authorize', params: authorization_params.merge(scope: client.scopes.split)
    expect(response).to have_http_status(:redirect)
    values = callback_values
    expect(values.fetch('state')).to eq('mobile-state')
    values.fetch('code')
  end

  def callback_values
    uri = URI(response.location)
    verify_callback(uri)
    URI.decode_www_form(uri.query).to_h
  end

  def verify_callback(uri)
    callback = uri.dup
    callback.query = nil
    expect(callback.to_s).to eq(client.redirect_uri)
  end

  def redeem(code, **overrides)
    post '/token', params: { grant_type: 'authorization_code', client_id: client.client_id,
                             redirect_uri: client.redirect_uri, code: code, code_verifier: verifier }.merge(overrides),
                   as: :json
  end
end
