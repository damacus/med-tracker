# frozen_string_literal: true

require 'rails_helper'
require 'omniauth/strategies/openid_connect'
require 'jwt'

RSpec.describe OmniAuth::Strategies::OpenIDConnect do
  let(:log_output) { StringIO.new }
  let(:key) { OpenSSL::PKey::RSA.generate(2048) }
  let(:claims) do
    { iss: issuer, aud: 'medtracker-web', sub: 'test-subject', nonce: 'expected-nonce',
      iat: Time.current.to_i, exp: 5.minutes.from_now.to_i }
  end
  let(:signed_token) { JWT.encode(claims, key, 'RS256', kid: 'test-key') }
  let(:handoff) { [] }

  def strategy
    described_class.new(
      lambda { |env|
        handoff << env['omniauth.auth']
        [204, {}, []]
      },
      name: :oidc, issuer: issuer, discovery: true, response_type: :code,
      client_options: { identifier: 'medtracker-web', secret: 'synthetic-client-secret',
                        redirect_uri: 'https://medtracker.example.test/auth/oidc/callback' }
    )
  end

  before do
    allow(OmniAuth.config).to receive(:logger).and_return(Logger.new(log_output))
    stub_request(:get, "#{issuer}/.well-known/openid-configuration").to_return(
      headers: { 'Content-Type' => 'application/json' },
      body: { issuer: issuer, authorization_endpoint: "#{issuer}/authorize", token_endpoint: "#{issuer}/token",
              userinfo_endpoint: "#{issuer}/userinfo", jwks_uri: "#{issuer}/jwks",
              response_types_supported: ['code'], subject_types_supported: ['public'],
              id_token_signing_alg_values_supported: ['RS256'] }.to_json
    )
    stub_request(:get, "#{issuer}/jwks").to_return(
      headers: { 'Content-Type' => 'application/json' },
      body: { keys: [JWT::JWK.new(key.public_key, 'test-key').export] }.to_json
    )
    stub_request(:post, "#{issuer}/token").to_return(
      headers: { 'Content-Type' => 'application/json' },
      body: { access_token: 'synthetic-provider-token', token_type: 'Bearer', id_token: signed_token }.to_json
    )
    stub_request(:get, "#{issuer}/userinfo").to_return(
      headers: { 'Content-Type' => 'application/json' }, body: { sub: 'test-subject' }.to_json
    )
  end

  it 'hands off a correctly signed and bound provider identity' do
    result = callback

    expect(result.first).to eq(204)
    expect(handoff.sole.uid).to eq('test-subject')
  end

  {
    'issuer' => { iss: 'https://other.example.test' },
    'audience' => { aud: 'other-client' },
    'expiry' => { exp: 1.hour.ago.to_i },
    'nonce' => { nonce: 'other-nonce' }
  }.each do |claim, invalid_values|
    context "with an invalid #{claim}" do
      let(:claims) { super().merge(invalid_values) }

      it 'does not hand an authenticated identity to the application' do
        callback

        expect(handoff).to be_empty
      end
    end
  end

  context 'with an invalid signature' do
    let(:signed_token) { JWT.encode(claims, OpenSSL::PKey::RSA.generate(2048), 'RS256', kid: 'test-key') }

    it 'does not hand an authenticated identity to the application' do
      callback

      expect(handoff).to be_empty
    end
  end

  def callback
    env = Rack::MockRequest.env_for('https://medtracker.example.test/auth/oidc/callback?code=synthetic-code&state=expected-state')
    env['rack.session'] = { 'omniauth.state' => 'expected-state', 'omniauth.nonce' => 'expected-nonce' }
    result = strategy.call(env)
    expect(log_output.string).not_to include(signed_token, 'synthetic-code', 'synthetic-provider-token',
                                             'synthetic-client-secret')
    result
  end

  def issuer
    'https://oidc.example.test'
  end
end
