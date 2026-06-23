# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuthTokenAuditLogger do
  subject(:audit_logger) { described_class.new }

  fixtures :accounts, :people, :users

  let(:account) { accounts(:jane_doe) }
  let(:user) { users(:jane) }
  let(:version) { PaperTrail::Version.where(item_type: 'AuthenticationToken').last }
  let(:security_event) { SecurityAuditEvent.order(:created_at).last }
  let(:sensitive_metadata) do
    {
      endpoint: 'https://example.com/push/subscriptions/raw-endpoint',
      user_agent: 'Raw Browser User Agent',
      device_name: 'RSpec iPhone',
      platform: 'ios',
      token: 'raw-token',
      token_digest: 'raw-token-digest',
      p256dh: 'raw-p256dh',
      auth: 'raw-auth',
      webauthn_id: 'raw-webauthn-id',
      public_key: 'raw-public-key',
      email: 'jane@example.com'
    }
  end

  before do
    PaperTrail.request.whodunnit = user.id
    PaperTrail.request.controller_info = { ip: '10.0.0.1', request_id: 'req-auth-001' }
  end

  after do
    PaperTrail.request.whodunnit = nil
    PaperTrail.request.controller_info = {}
  end

  describe '#record' do
    it 'creates a PaperTrail::Version with item_type AuthenticationToken' do
      expect do
        audit_logger.record(account: account, token_type: 'api_session', action: 'created')
      end.to change { PaperTrail::Version.where(item_type: 'AuthenticationToken').count }.by(1)
    end

    it 'creates a tenant-partitioned security audit event', :aggregate_failures do
      expect do
        record_security_audit_event
      end.to change(SecurityAuditEvent, :count).by(1)

      expect_tenant_security_event
    end

    def record_security_audit_event
      audit_logger.record(
        account: account,
        token_type: 'api_session',
        action: 'created',
        context: security_audit_context
      )
    end

    def security_audit_context
      {
        whodunnit: user.id,
        ip: '10.0.0.2',
        request_id: 'req-security-001',
        household_id: security_household.id,
        actor_membership_id: security_membership.id
      }
    end

    def expect_tenant_security_event
      expect(security_event).to have_attributes(
        household_id: security_household.id,
        actor_account_id: account.id,
        actor_membership_id: security_membership.id,
        event_type: 'auth_token/api_session/created',
        ip: '10.0.0.2',
        request_id: 'req-security-001'
      )
      expect(security_event.metadata).to include(
        'account_id' => account.id,
        'token_type' => 'api_session',
        'action' => 'created'
      )
    end

    def security_household
      @security_household ||= Household.create!(name: 'Security Audit Household', slug: 'security-audit-household')
    end

    def security_membership
      @security_membership ||= security_household.household_memberships.create!(
        account: account,
        role: :owner,
        status: :active,
        joined_at: Time.current
      )
    end

    it 'persists the event, account id, whodunnit, ip, and request_id', :aggregate_failures do
      audit_logger.record(account: account, token_type: 'api_session', action: 'created')

      expect(version.item_id).to eq(account.id)
      expect(version.event).to eq('auth_token/api_session/created')
      expect(version.whodunnit).to eq(user.id.to_s)
      expect(version.ip).to eq('10.0.0.1')
      expect(version.request_id).to eq('req-auth-001')
      expect(version_data).to include('account_id' => account.id, 'token_type' => 'api_session', 'action' => 'created')
    end

    it 'stores hashes and redacted metadata', :aggregate_failures do
      audit_logger.record(
        account: account,
        token_type: 'push_subscription',
        action: 'created',
        metadata: sensitive_metadata
      )

      expect(version_data['endpoint_hash']).to eq(Digest::SHA256.hexdigest(sensitive_metadata.fetch(:endpoint)))
      expect(version_data['user_agent_hash']).to eq(Digest::SHA256.hexdigest(sensitive_metadata.fetch(:user_agent)))
      expect(version_data).to include('device_name_present' => true, 'device_name_length' => 12, 'platform' => 'ios')
    end

    it 'does not store raw token material' do
      audit_logger.record(
        account: account,
        token_type: 'push_subscription',
        action: 'created',
        metadata: sensitive_metadata
      )

      expect(version.object).not_to match(sensitive_pattern)
    end

    it 'uses explicit audit context when provided', :aggregate_failures do
      audit_logger.record(
        account: account,
        token_type: 'api_session',
        action: 'rotated',
        context: { whodunnit: 123, ip: '127.0.0.1', request_id: 'req-explicit' }
      )

      expect(version.whodunnit).to eq('123')
      expect(version.ip).to eq('127.0.0.1')
      expect(version.request_id).to eq('req-explicit')
    end

    it 'does not raise when called without PaperTrail request context' do
      PaperTrail.request.whodunnit = nil
      PaperTrail.request.controller_info = {}

      expect do
        audit_logger.record(account: account, token_type: 'api_session', action: 'created')
      end.not_to raise_error
    end

    it 'silently rescues errors and logs them' do
      allow(PaperTrail::Version).to receive(:insert).and_raise(ActiveRecord::StatementInvalid)
      allow(Rails.logger).to receive(:error)

      expect do
        audit_logger.record(account: account, token_type: 'api_session', action: 'created')
      end.not_to raise_error

      expect(Rails.logger).to have_received(:error).with(/AuthTokenAuditLogger failed/)
    end
  end

  def version_data
    JSON.parse(version.object)
  end

  def sensitive_pattern
    Regexp.union(
      'raw-endpoint', 'Raw Browser User Agent', 'RSpec iPhone', 'raw-token', 'raw-p256dh',
      'raw-auth', 'raw-webauthn-id', 'raw-public-key', 'jane@example.com'
    )
  end
end
