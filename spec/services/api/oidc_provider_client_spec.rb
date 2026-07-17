# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::OidcProviderClient do
  let(:issuer) { 'https://issuer.example.test' }
  let(:discovery_url) { "#{issuer}/.well-known/openid-configuration" }
  let(:token_endpoint) { "#{issuer}/oauth/token" }
  let(:jwks_uri) { "#{issuer}/oauth/keys" }
  let(:redirect_uri) { 'https://mobile.example.test/oauth/callback' }

  before do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('OIDC_ISSUER_URL', nil).and_return(issuer)
    allow(ENV).to receive(:fetch).with('OIDC_DISCOVERY_URL', nil).and_return(discovery_url)
    allow(ENV).to receive(:fetch).with('OIDC_MOBILE_CLIENT_ID', nil).and_return('mobile-client')
    allow(ENV).to receive(:fetch).with('OIDC_MOBILE_REDIRECT_URIS', nil).and_return(redirect_uri)
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

  def stub_discovery
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
end
