# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::AuditLogs Rate Limiting' do
  include ActiveSupport::Testing::TimeHelpers

  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }

  around do |example|
    original_cache_store = Rack::Attack.cache.store
    original_enabled = Rack::Attack.enabled
    original_throttle = Rack::Attack.throttles.fetch('admin/audit_logs/ip')

    Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new
    Rack::Attack.enabled = true
    Rack::Attack.throttle('admin/audit_logs/ip', limit: 3, period: 1.minute, &original_throttle.block)

    example.run
  ensure
    Rack::Attack.throttles['admin/audit_logs/ip'] = original_throttle
    Rack::Attack.cache.store = original_cache_store
    Rack::Attack.enabled = original_enabled
  end

  before do
    freeze_time
    sign_in(admin)
  end

  after do
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
  end

  describe 'IP-based rate limiting on GET /admin/audit_logs' do
    it 'allows the third request, throttles the fourth, and includes retry metadata' do
      allow(Observability::DomainEventPublisher).to receive(:instrument).and_call_original

      3.times { get admin_audit_logs_path }
      expect(response).to have_http_status(:success)

      get admin_audit_logs_path
      expect(response).to have_http_status(:too_many_requests)
      expect(response.body).to include('Rate limit exceeded')
      expect(response.headers['Retry-After']).to be_present
      expect(response.headers['Retry-After'].to_i).to be > 0
      expect(Observability::DomainEventPublisher).to have_received(:instrument).with(
        'rack_attack.throttled',
        throttle: 'admin/audit_logs/ip',
        ip: kind_of(String)
      )

      get admin_root_path
      expect(response).to have_http_status(:success)
    end
  end

  describe 'IP-based rate limiting on GET /admin/audit_logs/:id' do
    let!(:version) do
      PaperTrail.request.whodunnit = admin.id
      PaperTrail.request.controller_info = paper_trail_info_for(admin)
      PaperTrail.request(enabled: true) do
        users(:jane).update!(active: false)
      end
      PaperTrail::Version.last
    end

    it 'throttles show endpoint as part of the same limit' do
      2.times { get admin_audit_logs_path }
      expect(response).to have_http_status(:success)

      get admin_audit_log_path(version)
      expect(response).to have_http_status(:success)

      get admin_audit_logs_path
      expect(response).to have_http_status(:too_many_requests)
    end
  end

  def paper_trail_info_for(user)
    household = ensure_api_household_for(user)
    account = Account.find_by(email: user.email_address)
    membership = household.household_memberships.find_by(account: account)
    { household_id: household.id, actor_membership_id: membership&.id }.compact
  end
end
