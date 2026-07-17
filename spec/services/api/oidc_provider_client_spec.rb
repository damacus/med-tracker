# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::OidcProviderClient do
  let(:issuer) { 'https://issuer.example.test' }
  let(:discovery_url) { "#{issuer}/.well-known/openid-configuration" }
  let(:token_endpoint) { "#{issuer}/oauth/token" }
  let(:jwks_uri) { "#{issuer}/oauth/keys" }
  let(:redirect_uri) { 'https://mobile.example.test/oauth/callback' }
  let(:signing_key) { OpenSSL::PKey::RSA.generate(2048) }

  before do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('OIDC_ISSUER_URL', nil).and_return(issuer)
    allow(ENV).to receive(:fetch).with('OIDC_DISCOVERY_URL', nil).and_return(discovery_url)
    allow(ENV).to receive(:fetch).with('OIDC_MOBILE_CLIENT_ID', nil).and_return('mobile-client')
    allow(ENV).to receive(:fetch).with('OIDC_MOBILE_REDIRECT_URIS', nil).and_return(redirect_uri)
    allow(ENV).to receive(:fetch).with('OIDC_ALLOWED_ENDPOINT_ORIGINS', nil).and_return(issuer)
    allow(Addrinfo).to receive(:getaddrinfo).and_return([Addrinfo.tcp('8.8.8.8', 443)])
  end

  it 'caches discovery with a bounded expiry while exchanging every authorization code' do
    cache = ActiveSupport::Cache::MemoryStore.new
    client = described_class.new(cache: cache)
    stub_discovery
    stub_token_response

    2.times { expect(exchange_code(client)).to eq('signed-id-token') }

    expect_cached_metadata
  end

  def expect_cached_metadata
    expect_endpoint_request_counts
    expect_cache_expiry
  end

  def expect_endpoint_request_counts
    expect(WebMock).to have_requested(:get, discovery_url).once
    expect(WebMock).to have_requested(:post, token_endpoint).twice
  end

  def expect_cache_expiry
    expect(described_class::DISCOVERY_CACHE_TTL).to eq(5.minutes)
    expect(described_class::JWKS_CACHE_TTL).to eq(5.minutes)
  end

  it 'rejects a redirect URI outside the server allowlist' do
    stub_discovery

    expect do
      described_class.new.exchange_code(
        authorization_code: 'authorization-code',
        code_verifier: 'a' * 64,
        redirect_uri: 'https://evil.example.test/callback'
      )
    end.to raise_error(described_class::Error)

    expect(WebMock).not_to have_requested(:post, token_endpoint)
  end

  it 'rejects HTTP issuer and discovery configuration outside test' do
    allow(ENV).to receive(:fetch).with('OIDC_ISSUER_URL', nil).and_return('http://issuer.example.test')
    allow(ENV).to receive(:fetch)
      .with('OIDC_DISCOVERY_URL', nil)
      .and_return('http://issuer.example.test/.well-known/openid-configuration')
    allow(Rails.env).to receive(:test?).and_return(false)

    expect do
      described_class.new.exchange_code(
        authorization_code: 'authorization-code',
        code_verifier: 'a' * 64,
        redirect_uri: redirect_uri
      )
    end.to raise_error(described_class::Error)
  end

  it 'maps a JSON array token response to a provider error' do
    stub_discovery
    stub_request(:post, token_endpoint).to_return(status: 200, body: [].to_json)

    expect { exchange_code(described_class.new) }.to raise_error(described_class::Error)
  end

  it 'maps non-object provider responses to provider errors' do
    [nil, 'invalid'].each do |payload|
      stub_discovery
      stub_request(:post, token_endpoint).to_return(status: 200, body: payload.to_json)

      expect { exchange_code(described_class.new(cache: memory_cache)) }.to raise_error(described_class::Error)
    end

    [[], nil, 'invalid'].each do |payload|
      stub_request(:get, discovery_url).to_return(status: 200, body: payload.to_json)

      expect { exchange_code(described_class.new(cache: memory_cache)) }.to raise_error(described_class::Error)
    end
  end

  it 'rejects provider objects with wrong field types' do
    stub_discovery
    stub_request(:post, token_endpoint).to_return(status: 200, body: { id_token: [] }.to_json)
    expect { exchange_code(described_class.new(cache: memory_cache)) }.to raise_error(described_class::Error)

    stub_discovery(token_endpoint: [])
    expect { exchange_code(described_class.new(cache: memory_cache)) }.to raise_error(described_class::Error)
  end

  it 'maps correctly signed non-object claims sets to provider errors' do
    stub_discovery
    stub_request(:get, jwks_uri).to_return(
      status: 200,
      body: { keys: [JWT::JWK.new(signing_key).export] }.to_json
    )
    client = described_class.new(cache: memory_cache)

    [[], nil, 'invalid'].each do |payload|
      expect { client.decode_id_token(signed_token(payload)) }.to raise_error(described_class::Error)
    end
  end

  it 'maps a correctly signed object with wrong claim types to a provider error' do
    stub_discovery
    stub_request(:get, jwks_uri).to_return(
      status: 200,
      body: { keys: [JWT::JWK.new(signing_key).export] }.to_json
    )
    payload = {
      'iss' => issuer,
      'aud' => 'mobile-client',
      'exp' => {},
      'iat' => Time.current.to_i,
      'nonce' => 'nonce',
      'sub' => 'subject'
    }

    expect do
      described_class.new(cache: memory_cache).decode_id_token(signed_token(payload))
    end.to raise_error(described_class::Error)
  end

  it 'rejects discovered endpoints outside configured origins' do
    endpoint = 'https://tokens.example.test/oauth/token'
    stub_discovery(token_endpoint: endpoint)
    stub_request(:post, endpoint).to_return(status: 200, body: { id_token: 'signed-id-token' }.to_json)

    expect { exchange_code(described_class.new(cache: memory_cache)) }.to raise_error(described_class::Error)
  end

  it 'allows an explicitly configured cross-origin endpoint' do
    endpoint = 'https://tokens.example.test/oauth/token'
    allow(ENV).to receive(:fetch).with('OIDC_ALLOWED_ENDPOINT_ORIGINS', nil)
      .and_return("#{issuer},https://tokens.example.test")
    stub_discovery(token_endpoint: endpoint)
    stub_request(:post, endpoint).to_return(status: 200, body: { id_token: 'signed-id-token' }.to_json)

    expect(exchange_code(described_class.new(cache: memory_cache))).to eq('signed-id-token')
  end

  it 'rejects userinfo, unexpected ports, and literal internal endpoints' do
    endpoints = [
      'https://user@issuer.example.test/oauth/token',
      'https://issuer.example.test:8443/oauth/token',
      'https://127.0.0.1/oauth/token',
      'https://169.254.169.254/oauth/token'
    ]

    endpoints.each do |endpoint|
      stub_discovery(token_endpoint: endpoint)
      stub_request(:post, endpoint).to_return(status: 200, body: { id_token: 'signed-id-token' }.to_json)

      expect { exchange_code(described_class.new(cache: memory_cache)) }.to raise_error(described_class::Error)
    end
  end

  it 'rejects a hostname that resolves to a private address' do
    endpoint = 'https://tokens.example.test/oauth/token'
    resolver = class_double(Addrinfo, getaddrinfo: [Addrinfo.tcp('10.0.0.5', 443)])
    allow(ENV).to receive(:fetch).with('OIDC_ALLOWED_ENDPOINT_ORIGINS', nil)
      .and_return("#{issuer},https://tokens.example.test")
    stub_discovery(token_endpoint: endpoint)

    expect do
      exchange_code(described_class.new(cache: memory_cache, resolver: resolver))
    end.to raise_error(described_class::Error)
  end

  it 'does not follow redirects from discovered endpoints' do
    redirect_target = 'https://issuer.example.test/internal-target'
    stub_discovery
    stub_request(:post, token_endpoint).to_return(status: 302, headers: { 'Location' => redirect_target })

    expect { exchange_code(described_class.new(cache: memory_cache)) }.to raise_error(described_class::Error)
    expect(WebMock).not_to have_requested(:get, redirect_target)
    expect(WebMock).not_to have_requested(:post, redirect_target)
  end

  def memory_cache = ActiveSupport::Cache::MemoryStore.new

  def stub_discovery(token_endpoint: self.token_endpoint, jwks_uri: self.jwks_uri)
    stub_request(:get, discovery_url).to_return(
      status: 200,
      body: {
        issuer: issuer,
        token_endpoint: token_endpoint,
        jwks_uri: jwks_uri,
        id_token_signing_alg_values_supported: ['RS256']
      }.to_json
    )
  end

  def stub_token_response
    stub_request(:post, token_endpoint).to_return(
      status: 200,
      body: { id_token: 'signed-id-token' }.to_json
    )
  end

  def exchange_code(client)
    client.exchange_code(
      authorization_code: 'authorization-code',
      code_verifier: 'a' * 64,
      redirect_uri: redirect_uri
    )
  end

  def signed_token(payload)
    jwk = JWT::JWK.new(signing_key)
    segments = [
      Base64.urlsafe_encode64({ alg: 'RS256', kid: jwk.kid }.to_json, padding: false),
      Base64.urlsafe_encode64(payload.to_json, padding: false)
    ]
    signing_input = segments.join('.')
    signature = signing_key.sign(OpenSSL::Digest.new('SHA256'), signing_input)
    [signing_input, Base64.urlsafe_encode64(signature, padding: false)].join('.')
  end
end
