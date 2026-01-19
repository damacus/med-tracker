# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::AuditLogs Rate Limiting' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:another_admin) { users(:john) }

  around do |example|
    original_cache_store = Rack::Attack.cache.store
    original_enabled = Rack::Attack.enabled

    fresh_cache = ActiveSupport::Cache::MemoryStore.new
    Rack::Attack.cache.store = fresh_cache
    Rack::Attack.enabled = true

    example.run
  ensure
    Rack::Attack.cache.store = original_cache_store
    Rack::Attack.enabled = original_enabled
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
      100.times do
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end

      get admin_audit_logs_path
      expect(response).to have_http_status(:too_many_requests)
      expect(response.body).to include('Rate limit exceeded')
    end

    it 'includes Retry-After header when throttled' do
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
      50.times do
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end

      50.times do
        get admin_audit_log_path(version)
        expect(response).to have_http_status(:success)
      end

      get admin_audit_logs_path
      expect(response).to have_http_status(:too_many_requests)
    end
  end

  describe 'user-based rate limiting' do
    let!(:version) do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request(enabled: true) do
        users(:jane).update!(role: :nurse)
      end
      PaperTrail::Version.last
    end

    it 'allows different users to have separate rate limit counters' do
      50.times do
        sign_in(admin)
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end

      50.times do
        sign_in(another_admin)
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end
    end

    it 'throttles at IP limit (100) before reaching user limit (200)' do
      100.times do
        get admin_audit_logs_path
        expect(response).to have_http_status(:success)
      end

      get admin_audit_logs_path
      expect(response).to have_http_status(:too_many_requests)
      expect(response.body).to include('Rate limit exceeded')
    end
  end

  describe 'rate limit does not affect other endpoints' do
    it 'does not throttle non-audit endpoints even after many requests' do
      101.times { get admin_audit_logs_path }
      expect(response).to have_http_status(:too_many_requests)

      get admin_root_path
      expect(response).to have_http_status(:success)
    end
  end

  describe 'rate limit logging' do
    it 'logs rate limit violations' do
      allow(Rails.logger).to receive(:warn)

      101.times { get admin_audit_logs_path }

      expect(Rails.logger).to have_received(:warn).with(%r{Rate limit exceeded: admin/audit_logs/ip})
    end
  end
end
