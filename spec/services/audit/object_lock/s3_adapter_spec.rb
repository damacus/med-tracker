# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::ObjectLock::S3Adapter do
  fixtures :accounts, :people, :users

  subject(:adapter) { described_class.new(configuration:, client:) }

  let(:configuration) do
    Audit::ObjectLock::Configuration.new(
      'AUDIT_WORM_BUCKET' => 'medtracker-audit',
      'AUDIT_WORM_REGION' => 'eu-west-2',
      'AUDIT_WORM_EXPECTED_OWNER' => '123456789012',
      'AUDIT_WORM_RETENTION_MODE' => 'GOVERNANCE',
      'AUDIT_WORM_SSE' => 'AES256'
    )
  end
  let(:client) { instance_double(Aws::S3::Client) }
  let(:entry) do
    event = Audit::Event.record!(household: users(:admin).person.household, event_type: 'audit.export.test')
    AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
  end

  it 'validates versioning, Object Lock, encryption, and expected ownership' do
    stub_valid_bucket

    expect { adapter.validate! }.not_to raise_error
    expect(client).to have_received(:head_bucket)
      .with(bucket: 'medtracker-audit', expected_bucket_owner: '123456789012')
  end

  it 'writes a deterministic content-addressed object with retention and a conditional create' do
    response = instance_double(Aws::S3::Types::PutObjectOutput, version_id: 'version-1')
    allow(client).to receive(:put_object).and_return(response)

    adapter.write(entry)

    expect(client).to have_received(:put_object) do |attributes|
      expect(attributes).to include(
        bucket: 'medtracker-audit', expected_bucket_owner: '123456789012', if_none_match: '*',
        checksum_algorithm: 'SHA256', object_lock_mode: 'GOVERNANCE', server_side_encryption: 'AES256'
      )
      expect(attributes[:key]).to end_with("#{entry.entry_hash.unpack1('H*')}.json")
      expect(attributes[:body]).to eq(Audit::ObjectLock::RecordSerializer.new(entry).body)
      expect(attributes[:object_lock_retain_until_date]).to eq(entry.retain_until)
    end
  end

  it 'rejects a bucket without Object Lock' do
    allow(client).to receive(:head_bucket)
    object_lock = instance_double(Aws::S3::Types::ObjectLockConfiguration, object_lock_enabled: nil)
    allow(client).to receive_messages(
      get_bucket_versioning: versioning_response,
      get_object_lock_configuration: object_lock_response(object_lock)
    )

    expect { adapter.validate! }.to raise_error(Audit::ObjectLock::ConfigurationError, /Object Lock/)
  end

  it 'treats a matching conditional-write conflict as an idempotent delivery' do
    serializer = Audit::ObjectLock::RecordSerializer.new(entry)
    allow(client).to receive(:put_object)
      .and_raise(Aws::S3::Errors::PreconditionFailed.new(nil, 'already exists'))
    allow(client).to receive(:head_object).and_return(
      instance_double(
        Aws::S3::Types::HeadObjectOutput,
        checksum_sha256: serializer.checksum_sha256_base64,
        metadata: {}, object_lock_retain_until_date: entry.retain_until,
        object_lock_mode: 'GOVERNANCE', version_id: 'existing-version'
      )
    )

    expect(adapter.write(entry).version_id).to eq('existing-version')
  end

  it 'refuses a conflicting object with the same content-addressed key' do
    allow(client).to receive(:put_object)
      .and_raise(Aws::S3::Errors::PreconditionFailed.new(nil, 'already exists'))
    allow(client).to receive(:head_object).and_return(
      instance_double(
        Aws::S3::Types::HeadObjectOutput,
        checksum_sha256: 'different', metadata: {}, object_lock_retain_until_date: entry.retain_until,
        object_lock_mode: 'GOVERNANCE', version_id: 'conflicting-version'
      )
    )

    expect { adapter.write(entry) }.to raise_error(Audit::ObjectLock::IntegrityError, /checksum/)
  end

  it 'refuses an existing object with shorter retention' do
    serializer = Audit::ObjectLock::RecordSerializer.new(entry)
    allow(client).to receive(:put_object)
      .and_raise(Aws::S3::Errors::PreconditionFailed.new(nil, 'already exists'))
    allow(client).to receive(:head_object).and_return(
      instance_double(
        Aws::S3::Types::HeadObjectOutput,
        checksum_sha256: serializer.checksum_sha256_base64,
        metadata: {}, object_lock_retain_until_date: entry.retain_until - 1.day,
        object_lock_mode: 'GOVERNANCE', version_id: 'short-retention'
      )
    )

    expect { adapter.write(entry) }.to raise_error(Audit::ObjectLock::IntegrityError, /retention is too short/)
  end

  it 'verifies the exact delivered object version' do
    serializer = Audit::ObjectLock::RecordSerializer.new(entry)
    delivery = delivery_for(serializer)
    allow(client).to receive_messages(
      head_object: valid_head(serializer, version_id: 'version-1'),
      list_object_versions: instance_double(
        Aws::S3::Types::ListObjectVersionsOutput, versions: [object_version(serializer, 'version-1')]
      )
    )

    expect(adapter.verify(entry, delivery:)).to be(true)
  end

  it 'rejects duplicate object versions' do
    serializer = Audit::ObjectLock::RecordSerializer.new(entry)
    delivery = delivery_for(serializer)
    versions = [object_version(serializer, 'version-1'), object_version(serializer, 'version-2')]
    allow(client).to receive_messages(
      head_object: valid_head(serializer, version_id: 'version-1'),
      list_object_versions: instance_double(Aws::S3::Types::ListObjectVersionsOutput, versions:)
    )

    expect { adapter.verify(entry, delivery:) }.to raise_error(Audit::ObjectLock::IntegrityError, /duplicate/)
  end

  def stub_valid_bucket
    object_lock = instance_double(Aws::S3::Types::ObjectLockConfiguration, object_lock_enabled: 'Enabled')
    allow(client).to receive_messages(
      head_bucket: instance_double(Aws::S3::Types::HeadBucketOutput),
      get_bucket_versioning: versioning_response,
      get_object_lock_configuration: object_lock_response(object_lock),
      get_bucket_encryption: encryption_response
    )
  end

  def versioning_response
    instance_double(Aws::S3::Types::GetBucketVersioningOutput, status: 'Enabled')
  end

  def object_lock_response(object_lock)
    instance_double(
      Aws::S3::Types::GetObjectLockConfigurationOutput,
      object_lock_configuration: object_lock
    )
  end

  def encryption_response
    default = instance_double(Aws::S3::Types::ServerSideEncryptionByDefault, sse_algorithm: 'AES256')
    rule = instance_double(
      Aws::S3::Types::ServerSideEncryptionRule,
      apply_server_side_encryption_by_default: default
    )
    configuration = instance_double(Aws::S3::Types::ServerSideEncryptionConfiguration, rules: [rule])
    instance_double(
      Aws::S3::Types::GetBucketEncryptionOutput,
      server_side_encryption_configuration: configuration
    )
  end

  def valid_head(serializer, version_id:)
    instance_double(
      Aws::S3::Types::HeadObjectOutput,
      checksum_sha256: serializer.checksum_sha256_base64,
      metadata: {}, object_lock_retain_until_date: entry.retain_until,
      object_lock_mode: 'GOVERNANCE', version_id:
    )
  end

  def delivery_for(serializer)
    instance_double(
      AuditExportDelivery,
      object_key: serializer.object_key, checksum_sha256: serializer.checksum_sha256,
      object_version_id: 'version-1', retention_mode: 'GOVERNANCE', retain_until: entry.retain_until
    )
  end

  def object_version(serializer, version_id)
    instance_double(Aws::S3::Types::ObjectVersion, key: serializer.object_key, version_id:)
  end
end
