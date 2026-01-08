# frozen_string_literal: true

require 'rails_helper'

# Tests for rate limiting on audit log endpoints
# These tests verify that Rack::Attack throttling is properly configured
RSpec.describe 'Admin::AuditLogs Rate Limiting' do
  fixtures :all

  let(:admin) { users(:admin) }

  # Enable Rack::Attack for these tests
  around do |example|
    # Store original state
    original_cache_store = Rack::Attack.cache.store

    # Enable Rack::Attack with a memory store for testing
    Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new
    Rack::Attack.enabled = true

    example.run

    # Clear throttle data after test
    Rack::Attack.cache.store.clear

    # Restore original state
    Rack::Attack.cache.store = original_cache_store
    Rack::Attack.enabled = false
  end

  before do
    sign_in(admin)
  end

  describe 'IP-based rate limiting on GET /admin/audit_logs' do
    it 'allows requests under the limit' do
      50.times do
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end
    end

    it 'throttles requests exceeding 100 per minute' do
      # Make 100 successful requests
      100.times do
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end

      # The 101st request should be throttled
      get admin_audit_logs_path
      expect(response).to have_http_status(:too_many_requests)
      expect(response.body).to include('Rate limit exceeded')
    end

    it 'includes Retry-After header when throttled' do
      # Exceed the limit
      101.times { get admin_audit_logs_path }

      expect(response.headers['Retry-After']).to be_present
      expect(response.headers['Retry-After'].to_i).to be > 0
    end
  end

  describe 'IP-based rate limiting on GET /admin/audit_logs/:id' do
    let!(:version) do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end
      PaperTrail::Version.last
    end

    it 'throttles show endpoint as part of the same limit' do
      # Make 50 requests to index
      50.times do
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end

      # Make 50 requests to show (should count towards same limit)
      50.times do
        get admin_audit_log_path(version)
        expect(response).to have_http_status(:success)
      end

      # The next request should be throttled
      get admin_audit_logs_path
      expect(response).to have_http_status(:too_many_requests)
    end
  end

  describe 'rate limit does not affect other endpoints' do
    it 'does not throttle non-audit endpoints even after many requests' do
      # Exhaust audit log limit
      101.times { get admin_audit_logs_path }
      expect(response).to have_http_status(:too_many_requests)

      # Other admin endpoints should still work
      get admin_root_path
      expect(response).to have_http_status(:success)
    end
  end

  describe 'rate limit logging' do
    it 'logs rate limit violations' do
      # Allow logger to capture warnings
      allow(Rails.logger).to receive(:warn)

      # Exceed the limit
      101.times { get admin_audit_logs_path }

      # Verify logging occurred
      expect(Rails.logger).to have_received(:warn).with(/Rate limit exceeded: admin\/audit_logs\/ip/)
    end
  end
end
