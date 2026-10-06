# frozen_string_literal: true

require 'rails_helper'

RSpec.describe StorageMigration::Backfill do
  subject(:backfill) do
    described_class.new(
      source_service_name:,
      destination_service_name:,
      blob_scope:,
      service_registry:,
      batch_size: 2
    )
  end

  before do
    blobs.zip(payloads).each do |blob, payload|
      source_service.upload(blob.key, StringIO.new(payload), checksum: blob.checksum)
    end
  end

  after do
    FileUtils.rm_rf(source_root)
    FileUtils.rm_rf(destination_root)
    ActiveStorage::Blob.where(id: blobs.map(&:id)).delete_all
  end

  it 'backfills Disk blobs to S3 by logical key in bounded batches' do
    allow(blob_scope).to receive(:find_in_batches).and_call_original

    result = backfill.call

    expect(blob_scope).to have_received(:find_in_batches).with(batch_size: 2)
    expect(result.to_h).to include(
      source_service_name: :persistent,
      destination_service_name: :s3,
      phase: :persistent_with_s3_mirror,
      processed_count: 3,
      verified_count: 3,
      failed_count: 0
    )
    expect(result.run_id).to match(Observability::CorrelationContext::UUID_PATTERN)
    expect_downloads(payloads)
  end

  it 'uses the same contract to backfill S3 blobs to Disk' do
    result = reverse_backfill

    expect(result.to_h).to include(processed_count: 3, verified_count: 3, failed_count: 0)
    expect_reverse_downloads
  ensure
    ActiveStorage::Blob.where(id: reverse_blobs.map(&:id)).delete_all
  end

  it 'accepts an existing valid destination without uploading it again' do
    first_blob = blobs.first
    destination_service.upload(first_blob.key, StringIO.new(payloads.first), checksum: first_blob.checksum)
    allow(destination_service).to receive(:upload).and_call_original

    result = backfill.call

    expect(result.failed_count).to be_zero
    expect(destination_service).not_to have_received(:upload).with(
      first_blob.key,
      anything,
      checksum: first_blob.checksum
    )
  end

  it 'reports missing source blobs without disclosing blob details' do
    source_service.delete(blobs.second.key)

    result = backfill.call

    expect(result.to_h).to include(processed_count: 3, verified_count: 2, failed_count: 1)
    expect(result.to_h.to_json).not_to include(blobs.second.key, blobs.second.filename.to_s)
  end

  it 'reports a corrupt existing destination instead of accepting it' do
    first_blob = blobs.first
    destination_service.upload(first_blob.key, StringIO.new('corrupt'), checksum: Digest::MD5.base64digest('corrupt'))

    result = backfill.call

    expect(result.to_h).to include(processed_count: 3, verified_count: 2, failed_count: 1)
  end

  it 'resumes safely after interruption and accepts already verified blobs' do
    upload_count = 0
    allow(destination_service).to receive(:upload).and_wrap_original do |method, *args, **keywords|
      upload_count += 1
      raise Interrupt if upload_count == 2

      method.call(*args, **keywords)
    end

    expect { backfill.call }.to raise_error(Interrupt)
    uploads_before_resume = upload_count

    result = backfill.call

    expect(upload_count - uploads_before_resume).to eq(2)
    expect(result.to_h).to include(processed_count: 3, verified_count: 3, failed_count: 0)
    expect_downloads(payloads)
  end

  it 'is idempotent when the same migration is retried' do
    first_result = backfill.call
    allow(destination_service).to receive(:upload).and_call_original

    second_result = backfill.call

    expect(first_result.failed_count).to be_zero
    expect(second_result.to_h).to include(processed_count: 3, verified_count: 3, failed_count: 0)
    expect(destination_service).not_to have_received(:upload)
  end

  def create_blob(payload:, service_name:)
    blob = ActiveStorage::Blob.new(
      key: SecureRandom.base58(28),
      filename: 'opaque.bin',
      content_type: 'application/octet-stream',
      byte_size: payload.bytesize,
      checksum: Digest::MD5.base64digest(payload),
      service_name:
    )
    blob.save!(validate: false)
    blob
  end

  def disk_service(root)
    ActiveStorage::Service::DiskService.new(root:)
  end

  def source_service_name
    :persistent
  end

  def destination_service_name
    :s3
  end

  def payloads
    %w[first second third]
  end

  def blobs
    @blobs ||= payloads.map { |payload| create_blob(payload:, service_name: source_service_name) }
  end

  def blob_scope
    @blob_scope ||= ActiveStorage::Blob.where(id: blobs)
  end

  def source_root
    @source_root ||= Dir.mktmpdir('storage-migration-source')
  end

  def destination_root
    @destination_root ||= Dir.mktmpdir('storage-migration-destination')
  end

  def service_registry
    @service_registry ||= { persistent: disk_service(source_root), s3: disk_service(destination_root) }
  end

  def source_service
    service_registry.fetch(source_service_name)
  end

  def destination_service
    service_registry.fetch(destination_service_name)
  end

  def reverse_backfill
    populate_reverse_source
    described_class.new(source_service_name: :s3, destination_service_name: :persistent,
                        blob_scope: ActiveStorage::Blob.where(id: reverse_blobs),
                        service_registry:, batch_size: 2).call
  end

  def reverse_blobs
    @reverse_blobs ||= payloads.map { |payload| create_blob(payload:, service_name: :s3) }
  end

  def populate_reverse_source
    reverse_blobs.zip(payloads).each do |blob, payload|
      service_registry.fetch(:s3).upload(blob.key, StringIO.new(payload), checksum: blob.checksum)
    end
  end

  def expect_reverse_downloads
    reverse_blobs.zip(payloads).each do |blob, payload|
      expect(service_registry.fetch(:persistent).download(blob.key)).to eq(payload)
    end
  end

  def expect_downloads(expected_payloads)
    blobs.zip(expected_payloads).each do |blob, payload|
      expect(destination_service.download(blob.key)).to eq(payload)
    end
  end
end
