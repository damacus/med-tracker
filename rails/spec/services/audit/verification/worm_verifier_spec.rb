# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Verification::WormVerifier do
  let(:adapter) { instance_double(Audit::ObjectLock::S3Adapter) }
  let(:record) { instance_double(AuditLedgerEntry, chain_key: 'global', sequence: 1) }
  let(:delivery) do
    instance_double(AuditExportDelivery, delivered?: true, pending?: false, failed?: false,
                                         export_record: record)
  end

  it 'validates every delivered object and returns a valid result' do
    allow(adapter).to receive(:verify).with(record, delivery:).and_return(true)

    result = described_class.new(records: [record], deliveries: [delivery], adapter:).call

    expect(result).to be_valid
    expect(result.checked_objects).to eq(1)
  end

  it 'reports missing outbox rows and undelivered evidence' do
    pending = instance_double(AuditExportDelivery, delivered?: false, pending?: true, failed?: false,
                                                   export_record: record)

    missing = described_class.new(records: [record], deliveries: [], adapter:).call
    undelivered = described_class.new(records: [record], deliveries: [pending], adapter:).call

    expect(missing.issue_codes).to include('worm_delivery_missing')
    expect(undelivered.issue_codes).to include('worm_delivery_pending')
  end

  it 'reports remote checksum, version, retention, or duplicate divergence' do
    allow(adapter).to receive(:verify)
      .and_raise(Audit::ObjectLock::IntegrityError, 'existing audit object checksum does not match')

    result = described_class.new(records: [record], deliveries: [delivery], adapter:).call

    expect(result.issue_codes).to include('worm_object_invalid')
    expect(result.issues.first.message).to eq('existing audit object checksum does not match')
  end

  it 'treats object-store availability failures as runtime errors' do
    allow(adapter).to receive(:verify).and_raise(Audit::ObjectLock::RetryableError, 'unavailable')

    expect do
      described_class.new(records: [record], deliveries: [delivery], adapter:).call
    end.to raise_error(Audit::ObjectLock::RetryableError)
  end
end
