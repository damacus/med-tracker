# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Storage::RestoreVerifier do
  subject(:verification) do
    described_class.new(
      attachment_id: attachment.id,
      service_registry:,
      access_verifier:
    ).call
  end

  let(:person) { create(:person) }
  let(:attachment) { person.avatar.attachment }
  let(:service_registry) { ActiveStorage::Blob.services }
  let(:access_verifier) do
    ->(attachment:) { { authorized_retrieval: attachment.present?, cross_household_denied: true } }
  end

  before do
    person.avatar.attach(io: StringIO.new('restored avatar'), filename: 'avatar.png', content_type: 'image/png')
  end

  after do
    person.avatar.purge
  end

  it 'verifies that the restored blob exists and matches its stored checksum' do
    result = verification

    expect(result.to_h).to include(
      attachment_id: attachment.id,
      blob_id: attachment.blob_id,
      byte_size: 'restored avatar'.bytesize,
      required_backends: [:disk],
      authorized_retrieval: true,
      cross_household_denied: true
    )
  end

  it 'verifies an S3-selected blob without using the current default service' do
    s3_root = Dir.mktmpdir('restore-verifier-s3')
    s3_service = ActiveStorage::Service::DiskService.new(root: s3_root)
    blob = attachment.blob
    s3_service.upload(blob.key, StringIO.new('restored avatar'), checksum: blob.checksum)
    registry = storage_registry(s3_service)
    allow(ActiveStorage::Blob).to receive(:services).and_return(registry)
    blob.update!(service_name: 's3')
    result = described_class.new(attachment_id: attachment.id, service_registry: registry, access_verifier:).call

    expect(result.required_backends).to eq([:s3])
  ensure
    blob&.update!(service_name: 'test')
    FileUtils.remove_entry(s3_root) if s3_root && File.exist?(s3_root)
  end

  def storage_registry(s3_service)
    test_service = service_registry.fetch('test')
    { s3: s3_service, test: test_service, 's3' => s3_service, 'test' => test_service }
  end

  it 'rejects a database attachment whose stored object is missing' do
    allow(service_registry.fetch(attachment.blob.service_name)).to receive(:exist?)
      .with(attachment.blob.key).and_return(false)

    expect { verification }
      .to raise_error(described_class::VerificationError, /stored object is missing/)
  end

  it 'rejects a restored object that fails the Active Storage integrity check' do
    allow(ActiveStorage::Attachment).to receive(:find_by).with(id: attachment.id).and_return(attachment)
    service = service_registry.fetch(attachment.blob.service_name)
    allow(service).to receive(:open).and_raise(ActiveStorage::IntegrityError)

    expect { verification }
      .to raise_error(described_class::VerificationError, /checksum/)
  end

  it 'requires an attachment from the restored database' do
    expect { described_class.new(attachment_id: 'missing', service_registry:, access_verifier:).call }
      .to raise_error(described_class::VerificationError, /attachment/)
  end

  it 'rejects incomplete authorization evidence without exposing household details' do
    verifier = ->(attachment:) { { authorized_retrieval: attachment.present?, cross_household_denied: false } }

    expect do
      described_class.new(
        attachment_id: attachment.id,
        service_registry:,
        access_verifier: verifier
      ).call
    end.to raise_error(described_class::VerificationError, 'cross_household_denial_failed')
  end
end
