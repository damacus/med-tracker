# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::OidcJwks do
  subject(:loader) { described_class.new(issuer: issuer, cache: cache) }

  let(:issuer) { 'https://issuer.example.test' }
  let(:jwks_uri) { 'https://issuer.example.test/oauth/v2/keys' }
  let(:signing_key) { OpenSSL::PKey::RSA.generate(2048) }
  let(:jwk) { JWT::JWK.new(signing_key, kid: 'current-key', use: 'sig', alg: 'RS256') }
  let(:cache) { ActiveSupport::Cache::MemoryStore.new }

  before do
    stub_request(:get, "#{issuer}/.well-known/openid-configuration")
      .to_return(
        status: 200,
        body: { issuer: issuer, jwks_uri: jwks_uri }.to_json,
        headers: { 'Content-Type' => 'application/json' }
      )
    stub_request(:get, jwks_uri)
      .to_return(
        status: 200,
        body: JWT::JWK::Set.new(jwk).export.to_json,
        headers: { 'Content-Type' => 'application/json' }
      )
  end

  it 'loads signing keys from the issuer discovery document' do
    expect(loader.call.first[:kid]).to eq('current-key')
  end

  it 'caches keys and refreshes them when a token key is not found' do
    loader.call
    loader.call

    expect(WebMock).to have_requested(:get, jwks_uri).once

    loader.call(kid_not_found: true)

    expect(WebMock).to have_requested(:get, jwks_uri).twice
  end

  it 'rejects discovery documents for a different issuer' do
    stub_request(:get, "#{issuer}/.well-known/openid-configuration")
      .to_return(
        status: 200,
        body: { issuer: 'https://evil.example.test', jwks_uri: jwks_uri }.to_json,
        headers: { 'Content-Type' => 'application/json' }
      )

    expect { loader.call }.to raise_error(described_class::Error, 'OIDC discovery issuer does not match')
  end
end
