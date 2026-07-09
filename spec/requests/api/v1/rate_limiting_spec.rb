# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 rate limiting' do
  include ActiveSupport::Testing::TimeHelpers

  around do |example|
    original_cache_store = Rack::Attack.cache.store
    original_enabled = Rack::Attack.enabled

    Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new
    Rack::Attack.enabled = true

    example.run
  ensure
    Rack::Attack.cache.store = original_cache_store
    Rack::Attack.enabled = original_enabled
  end

  before { freeze_time }

  it 'returns JSON retry metadata for throttled API routes' do
    10.times do
      post api_v1_auth_oidc_exchange_path,
           params: { id_token: 'invalid', nonce: 'nonce', code_verifier: 'verifier' },
           as: :json
    end

    post api_v1_auth_oidc_exchange_path,
         params: { id_token: 'invalid', nonce: 'nonce', code_verifier: 'verifier' },
         as: :json

    expect(response).to have_http_status(:too_many_requests)
    expect(response.media_type).to eq('application/json')
    expect(response.parsed_body.dig('error', 'code')).to eq('rate_limited')
    expect(response.headers['Retry-After'].to_i).to be > 0
    expect(response.headers['ratelimit-limit']).to eq('10')
    expect(response.headers['ratelimit-remaining']).to eq('0')
  end
end
