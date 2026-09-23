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

  it 'resumes mobile authorization after an expired browser login' do
    sign_in(user)
    user.person.account.account_active_session_keys.sole.update!(last_use: 31.days.ago)

    get '/authorize', params: authorization_params
    expect(response).to redirect_to('/login')

    post '/login', params: { email: user.person.account.email, password: 'password' }
    expect(URI(response.location).path).to eq('/authorize')
    expect(URI.decode_www_form(URI(response.location).query).to_h).to include(
      'client_id' => client.client_id, 'redirect_uri' => client.redirect_uri, 'state' => 'mobile-state'
    )
    follow_redirect!
    redeem(authorization_code)
    expect(response).to have_http_status(:ok)
  end

  it 'resumes mobile authorization after a rejected login form' do
    original_value = ActionController::Base.allow_forgery_protection
    ActionController::Base.allow_forgery_protection = true

    get '/authorize', params: authorization_params
    expect(response).to redirect_to('/login')

    post '/login', params: { email: user.person.account.email, password: 'password' }
    expect(response).to redirect_to('/login')

    get '/login'
    token = response.parsed_body.at_css('form[action="/login"] input[name="authenticity_token"]')[:value]
    post '/login', params: { email: user.person.account.email, password: 'password', authenticity_token: token }
    expect(URI(response.location).path).to eq('/authorize')
  ensure
    ActionController::Base.allow_forgery_protection = original_value
  end

  ['https://example.test/authorize?state=foreign', '/profile?state=foreign'].each do |return_path|
    it "does not restore #{return_path} after a rejected login form" do
      original_value = ActionController::Base.allow_forgery_protection
      ActionController::Base.allow_forgery_protection = true
      ensure_api_household_for(user)

      get '/login'
      session[:login_redirect] = return_path
      post '/login', params: { email: user.person.account.email, password: 'password' }
      expect(response).to redirect_to('/login')

      get '/login'
      token = response.parsed_body.at_css('form[action="/login"] input[name="authenticity_token"]')[:value]
      post '/login', params: { email: user.person.account.email, password: 'password', authenticity_token: token }
      expect(URI(response.location).path).to eq('/households/fixture-household/dashboard')
    ensure
      ActionController::Base.allow_forgery_protection = original_value
    end
  end

  it 'returns consent to the registered iOS staging callback' do
    client.update!(client_id: 'medtracker-ios-staging',
                   redirect_uri: 'io.damacus.medtracker.staging:/oauth2redirect')
    ensure_api_household_for(user)
    get '/authorize', params: authorization_params
    post '/login', params: { email: user.person.account.email, password: 'password' }
    follow_redirect!
    expect(response.body).to include('authorize-form')

    code = authorization_code
    expect(response.location).to start_with('io.damacus.medtracker.staging:/oauth2redirect?')
    expect(callback_values).to include('code' => code, 'state' => 'mobile-state')
    redeem(code)
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
    issued_hashes = OauthGrant.last.attributes.slice('token_hash', 'refresh_token_hash')
    redeem(code)

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
    expect(OauthGrant.last.attributes.slice('token_hash', 'refresh_token_hash')).to eq(issued_hashes)
  end

  {
    'missing challenge' => { code_challenge: nil },
    'plain challenge' => { code_challenge: 'mobile-code-verifier-with-more-than-forty-three-characters',
                           code_challenge_method: 'plain' }
  }.each do |invalid_challenge, overrides|
    it "rejects authorization with a #{invalid_challenge}" do
      sign_in(user)

      expect do
        post '/authorize', params: authorization_params.merge(scope: client.scopes.split).merge(overrides)
      end.not_to change(OauthGrant, :count)

      expect(response).to have_http_status(:redirect)
      expect(callback_values).to include('error' => 'invalid_request', 'state' => 'mobile-state')
      expect(callback_values).not_to have_key('code')
    end
  end

  it 'rejects a code issued to another registered mobile client' do
    sign_in(user)
    code = authorization_code
    other_client = client.dup
    other_client.update!(client_id: 'other-mobile-client')

    redeem(code, client_id: other_client.client_id)

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
    redeem(code)
    expect(response).to have_http_status(:ok)
  end

  it 'rejects an expired authorization code' do
    sign_in(user)
    code = authorization_code
    OauthGrant.last.update!(expires_in: 1.second.ago)

    redeem(code)

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
    expect(OauthGrant.last.token_hash).to be_nil
  end

  it 'rejects an existing mobile code with a plain challenge' do
    sign_in(user)
    code = authorization_code
    grant = OauthGrant.last
    grant.update!(code_challenge: verifier, code_challenge_method: 'plain')

    redeem(code)

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
    expect(grant.reload.token_hash).to be_nil
  end

  it 'filters credential inputs from failed token request diagnostics' do
    redeem('synthetic-authorization-code', code_verifier: 'synthetic-secret-verifier')

    expect(response).to have_http_status(:bad_request)
    expect(request.filtered_parameters).to include('code' => '[FILTERED]', 'code_verifier' => '[FILTERED]')
    expect(response.body).not_to include('synthetic-authorization-code', 'synthetic-secret-verifier')
  end

  it 'rolls back failed credential persistence before allowing one retry' do
    sign_in(user)
    code = authorization_code
    grant = OauthGrant.last
    connection = ActiveRecord::Base.connection
    connection.execute('ALTER TABLE oauth_grants ADD CONSTRAINT test_token_write_failure ' \
                       'CHECK (token_hash IS NULL) NOT VALID')
    audit_count = PaperTrail::Version.count

    expect { redeem(code) }.to raise_error(Sequel::CheckConstraintViolation)
    expect(grant.reload).to have_attributes(code: code, token_hash: nil, refresh_token_hash: nil)
    expect(PaperTrail::Version.count).to eq(audit_count)

    connection.execute('ALTER TABLE oauth_grants DROP CONSTRAINT test_token_write_failure')
    redeem(code)
    expect(response).to have_http_status(:ok)
    headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }
    get '/api/v1/auth/households', headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    redeem(code)
    expect(response).to have_http_status(:bad_request)
  ensure
    connection&.execute('ALTER TABLE oauth_grants DROP CONSTRAINT IF EXISTS test_token_write_failure')
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

    second_membership.update!(status: :revoked)
    get "/api/v1/households/#{second.id}/admin/settings", headers: headers, as: :json
    expect(response).to have_http_status(:forbidden)
    expect(response.body).not_to include(second.name)
    get "/api/v1/households/#{first.id}/admin/settings", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
  end

  it 'rejects refresh after the configured inactivity window' do
    sign_in(user)
    redeem(authorization_code)
    refresh_token = response.parsed_body.fetch('refresh_token')
    OauthGrant.last.update!(last_used_at: 31.days.ago)

    refresh(refresh_token)

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).not_to have_key('access_token')
  end

  it 'does not resolve a person from another authorised household' do
    sign_in(user)
    redeem(authorization_code)
    headers = { 'Authorization' => "Bearer #{response.parsed_body.fetch('access_token')}" }
    first = user.person.household
    second = Household.create!(name: 'Other authorised household', slug: 'other-authorised-household')
    membership = second.household_memberships.create!(account: user.person.account, role: :owner, status: :active)
    person = second.people.create!(name: 'Private other person', date_of_birth: 30.years.ago.to_date,
                                   person_type: :adult, has_capacity: true)
    PersonAccessGrant.create!(household: second, household_membership: membership, person: person,
                              access_level: :view, relationship_type: :family_member)

    get "/api/v1/households/#{first.id}/people/#{person.id}", headers: headers, as: :json

    expect(response).to have_http_status(:not_found)
    expect(response.body).not_to include(person.name)
    get "/api/v1/households/#{second.id}/people/#{person.id}", headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(person.id)
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

    refresh(old_refresh)
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('refresh_token')).not_to eq(old_refresh)
    expect(grant.reload).to have_attributes(last_used_at: activity, authenticated_at: authenticated_at)
    refresh(old_refresh)
    expect(response).to have_http_status(:bad_request)
  end

  {
    'missing verifier' => { code_verifier: nil },
    'mismatched callback' => { redirect_uri: 'io.damacus.medtracker:/other' }
  }.each do |invalid_parameter, overrides|
    it "rejects a #{invalid_parameter} without consuming a valid code" do
      sign_in(user)
      code = authorization_code

      redeem(code, **overrides)
      expect(response).to have_http_status(:bad_request)
      expect(response.parsed_body).not_to have_key('access_token')

      redeem(code)
      expect(response).to have_http_status(:ok)
    end
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

  def refresh(refresh_token)
    post '/token', params: { grant_type: 'refresh_token', client_id: client.client_id,
                             refresh_token: refresh_token }, as: :json
  end
end
