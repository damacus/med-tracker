# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SupportAccessSessions::ExpiryProcessor do
  let(:account) { Account.create!(email: 'expiry-operator@example.test', status: :verified) }
  let(:platform_admin) { PlatformAdmin.create!(account: account) }
  let(:household) { Household.create!(name: 'Expiry Household', slug: 'expiry-household') }
  let!(:support_session) do
    SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: 1.hour.ago,
      starts_at: 1.hour.ago,
      expires_at: 30.minutes.ago
    )
  end

  it 'marks naturally expired sessions and records one sanitized audit event', :aggregate_failures do
    expect { described_class.call }
      .to change { SecurityAuditEvent.where(event_type: 'support_access_session.expired').count }.by(1)

    event = SecurityAuditEvent.where(event_type: 'support_access_session.expired').sole
    expect(support_session.reload.expired_at).to be_present
    expect(event).to have_attributes(household: household, actor_account: account)
    expect(event.metadata).to include(
      'support_access_session_id' => support_session.id,
      'outcome' => 'expired'
    )
    expect(event.metadata).not_to include('reason', 'email', 'token')
    expect(event.metadata.to_json).not_to include(support_session.reason, account.email)
  end

  it 'is idempotent when expiry processing is retried' do
    2.times { described_class.call }

    expect(support_session.reload.expired_at).to be_present
    expect(SecurityAuditEvent.where(event_type: 'support_access_session.expired').count).to eq(1)
  end

  it 'allows only the atomic claim winner to audit during concurrent processing' do
    serialize_support_session_lock
    allow(TenantContext).to receive(:with).and_yield
    allow(Audit::Event).to receive(:record!)

    results = concurrent_expiry_results

    expect(results).to contain_exactly(true, false)
    expect(Audit::Event).to have_received(:record!).once
  end

  it 'rolls back the expiry marker when audit persistence fails so a retry can succeed' do
    allow(Audit::Event).to receive(:record!).and_raise(ActiveRecord::RecordInvalid.new(SecurityAuditEvent.new))

    expect { described_class.call }.to raise_error(ActiveRecord::RecordInvalid)
    expect(support_session.reload.expired_at).to be_nil

    allow(Audit::Event).to receive(:record!).and_call_original
    expect { described_class.call }
      .to change { SecurityAuditEvent.where(event_type: 'support_access_session.expired').count }.by(1)
  end

  it 'does not turn an explicitly ended session into a natural expiry' do
    support_session.update!(ended_at: 40.minutes.ago)

    expect { described_class.call }
      .not_to(change { SecurityAuditEvent.where(event_type: 'support_access_session.expired').count })

    expect(support_session.reload.expired_at).to be_nil
  end

  def serialize_support_session_lock
    mutex = Mutex.new
    load_support_session_associations
    stub_support_session_lock(mutex)
  end

  def load_support_session_associations
    support_session.platform_admin.account
    support_session.household
  end

  def stub_support_session_lock(mutex)
    allow(support_session).to receive(:with_lock) { |&block| mutex.synchronize(&block) }
    allow(support_session).to receive(:update!) { |attributes| support_session.assign_attributes(attributes) }
  end

  def concurrent_expiry_results
    2.times.map do
      Thread.new { described_class.new.send(:expire, support_session) }
    end.map(&:value)
  end
end
