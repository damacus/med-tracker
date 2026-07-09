# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::ObjectLock::DeliveryExporter do
  fixtures :accounts, :people, :users

  let(:adapter) { instance_double(Audit::ObjectLock::S3Adapter) }
  let(:exporter) { described_class.new(adapter:) }
  let(:delivery) do
    event = Audit::Event.record!(household: users(:admin).person.household, event_type: 'audit.delivery.test')
    AuditExportDelivery.find_by!(audit_ledger_entry: AuditLedgerEntry.find_by!(source_id: event.id,
                                                                               source_table: 'security_audit_events'))
  end
  let(:result) do
    Audit::ObjectLock::WriteResult.new(
      object_key: 'audit/v1/object.json', checksum_sha256: 'checksum', version_id: 'version-1',
      retention_mode: 'GOVERNANCE', retain_until: 10.years.from_now
    )
  end

  it 'marks a successfully persisted object as delivered' do
    allow(adapter).to receive(:write).with(delivery.audit_ledger_entry).and_return(result)

    expect(exporter.deliver(delivery)).to be(true)
    expect(delivery.reload).to have_attributes(
      status: 'delivered', attempts: 1, object_key: 'audit/v1/object.json', checksum_sha256: 'checksum',
      object_version_id: 'version-1', retention_mode: 'GOVERNANCE'
    )
  end

  it 'schedules retryable failures without discarding the delivery' do
    allow(adapter).to receive(:write).and_raise(Audit::ObjectLock::RetryableError, 'object store unavailable')

    expect(exporter.deliver(delivery)).to be(false)
    expect(delivery.reload).to have_attributes(status: 'pending', attempts: 1, last_error_code: 'retryable')
    expect(delivery.next_attempt_at).to be_future
  end

  it 'marks invalid retention or configuration as failed for operator action' do
    allow(adapter).to receive(:write).and_raise(Audit::ObjectLock::ConfigurationError, 'wrong retention mode')

    expect(exporter.deliver(delivery)).to be(false)
    expect(delivery.reload).to have_attributes(status: 'failed', attempts: 1, last_error_code: 'configuration')
  end
end
