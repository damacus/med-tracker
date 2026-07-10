# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'SMART OAuth authorization' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }
  let(:membership) do
    user.person.household.household_memberships.find_or_create_by!(
      account: user.person.account,
      person: user.person
    ).tap do |record|
      record.update!(role: :member, status: :active)
    end
  end
  let(:code_verifier) { 'smart-code-verifier-with-more-than-forty-three-characters' }
  let!(:oauth_application) do
    OauthApplication.create!(
      name: 'Trusted SMART client',
      client_id: 'smart-client',
      redirect_uri: 'https://client.example/callback',
      scopes: 'launch/patient patient/*.rs offline_access'
    )
  end
  let(:authorization_params) do
    {
      response_type: 'code',
      client_id: oauth_application.client_id,
      redirect_uri: oauth_application.redirect_uri,
      scope: oauth_application.scopes,
      state: 'opaque-state',
      code_challenge: Base64.urlsafe_encode64(Digest::SHA256.digest(code_verifier), padding: false),
      code_challenge_method: 'S256'
    }
  end

  before do
    membership
    sign_in user
  end

  it 'renders consent for a registered redirect URI and valid PKCE challenge' do
    get '/authorize', params: authorization_params

    expect(response).to have_http_status(:ok), response.body
    expect(response.body).to include(
      oauth_application.name,
      'patient/*.rs',
      user.person.household.name,
      user.person.name
    )
  end

  it 'exchanges an authorization code with PKCE and stores only token digests' do
    payload = issue_tokens
    grant = OauthGrant.order(:id).last
    audit_event = SecurityAuditEvent.find_by!(event_type: 'smart_oauth.consent_granted')
    expect(response).to have_http_status(:ok)
    expect(payload).to include('access_token', 'refresh_token')
    expect(grant).to have_attributes(
      household_membership_id: membership.id,
      person_id: user.person.id,
      permissions_version: membership.permissions_version,
      scopes: oauth_application.scopes,
      token: nil,
      refresh_token: nil
    )
    expect(grant.token_hash).to eq(OauthGrant.digest(payload.fetch('access_token')))
    expect(audit_event.metadata).to include(
      'oauth_application_id' => oauth_application.id,
      'household_membership_id' => membership.id,
      'person_id' => user.person.id,
      'scopes' => oauth_application.scopes
    )
    expect(audit_event.metadata.keys).not_to include('token', 'refresh_token', 'code')
  end

  it 'rotates refresh tokens and revokes the resulting grant' do
    payload = issue_tokens

    post '/token', params: {
      grant_type: 'refresh_token',
      client_id: oauth_application.client_id,
      refresh_token: payload.fetch('refresh_token')
    }, as: :json

    refreshed = response.parsed_body
    expect(response).to have_http_status(:ok), response.body
    expect(refreshed.fetch('refresh_token')).not_to eq(payload.fetch('refresh_token'))

    post '/logout'
    post '/revoke', params: {
      client_id: oauth_application.client_id,
      token: refreshed.fetch('access_token'),
      token_type_hint: 'access_token'
    }, as: :json

    expect(response).to have_http_status(:ok)
    expect(OauthGrant.order(:id).last.revoked_at).to be_present
    expect(SecurityAuditEvent.find_by!(event_type: 'smart_oauth.token_revoked').metadata.keys)
      .not_to include('token', 'refresh_token', 'code')
  end

  it 'rejects an unregistered redirect URI' do
    get '/authorize', params: authorization_params.merge(redirect_uri: 'https://attacker.example/callback')

    expect(response).to have_http_status(:ok)
    expect(response.body).to include("Invalid or missing 'redirect_uri'")
    expect(response.headers['Location']).to be_nil
  end

  it 'rejects authorization without PKCE' do
    get '/authorize', params: authorization_params.except(:code_challenge, :code_challenge_method)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include(
      'action="https://client.example/callback"',
      'name="error" value="invalid_request"',
      'name="error_description" value="code challenge required"'
    )
  end

  it 'rejects a code exchange without the PKCE verifier' do
    post '/authorize', params: authorization_params.merge(scope: oauth_application.scopes.split)
    code = response.parsed_body.at_css('input[name="code"]')['value']

    post '/token', params: {
      grant_type: 'authorization_code',
      client_id: oauth_application.client_id,
      redirect_uri: oauth_application.redirect_uri,
      code: code
    }, as: :json

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body).to include('error' => 'invalid_request')
  end

  it 'provides a denied-consent response preserving state' do
    get '/authorize', params: authorization_params

    denial_url = response.parsed_body.at_css('a.btn-outline-danger')['href']
    expect(denial_url).to include('error=access_denied', 'state=opaque-state')
  end

  def issue_tokens
    post '/token', params: {
      grant_type: 'authorization_code',
      client_id: oauth_application.client_id,
      redirect_uri: oauth_application.redirect_uri,
      code: authorization_code,
      code_verifier: code_verifier
    }, as: :json

    response.parsed_body
  end

  def authorization_code
    post '/authorize', params: authorization_params.merge(scope: oauth_application.scopes.split)
    response.parsed_body.at_css('input[name="code"]')['value']
  end
end
