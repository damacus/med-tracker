# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::ArchiveStore do
  let(:services) do
    {
      persistent: ActiveStorage::Service::DiskService.new(root: persistent_root),
      s3: ActiveStorage::Service::DiskService.new(root: s3_root)
    }
  end
  let(:store) { described_class.new(service_registry: services) }
  let(:import_run) { NhsDmdImport.create!(uploaded_filename: 'patient-medications.zip') }
  let(:upload) do
    Tempfile.create(['release', '.zip']).tap do |file|
      file.write(payload)
      file.rewind
    end
  end

  after do
    upload.close
    FileUtils.rm_f(upload.path)
    FileUtils.rm_rf(persistent_root)
    FileUtils.rm_rf(s3_root)
  end

  it 'persists and verifies an opaque durable reference before returning' do
    reference = store.persist(import_run:, uploaded_file: upload, service_name: :persistent)

    expect(reference.to_h).to include(
      service_name: 'persistent',
      checksum: Digest::MD5.base64digest(payload),
      byte_size: payload.bytesize
    )
    expect(reference.key).not_to include(import_run.uploaded_filename, import_run.id.to_s)
    expect(import_run.reload).to have_attributes(
      archive_service_name: 'persistent',
      archive_key: reference.key,
      archive_checksum: reference.checksum,
      archive_byte_size: payload.bytesize,
      archive_path: nil
    )
    expect(services.fetch(:persistent).download(reference.key)).to eq(payload)
  end

  it 'resolves retry-safe reads through either declared backend' do
    %i[persistent s3].each do |service_name|
      run = NhsDmdImport.create!(uploaded_filename: 'release.zip')
      store.persist(import_run: run, uploaded_file: upload, service_name:)

      first = store.open(run) { |path| File.binread(path) }
      second = store.open(run) { |path| File.binread(path) }

      expect(first).to eq(payload)
      expect(second).to eq(payload)
      expect(services.fetch(service_name)).to exist(run.archive_key)
    end
  end

  it 'preserves errors raised by the archive consumer' do
    store.persist(import_run:, uploaded_file: upload, service_name: :persistent)

    expect do
      store.open(import_run) { raise NhsDmd::ReleaseArchiveImport::Error, 'bad zip' }
    end.to raise_error(NhsDmd::ReleaseArchiveImport::Error, 'bad zip')
  end

  it 'reports a missing configured service through the archive store contract' do
    registry = ActiveStorage::Service::Registry.new({})
    unavailable_store = described_class.new(service_registry: registry)
    import_run.update!(
      archive_service_name: 'missing',
      archive_key: 'missing-key',
      archive_checksum: Digest::MD5.base64digest('missing'),
      archive_byte_size: 7
    )

    expect do
      unavailable_store.open(import_run) { nil }
    end.to raise_error(NhsDmd::ArchiveStore::Error, 'archive_service_unavailable')
  end

  it 'deletes a durable archive only after the import becomes terminal' do
    store.persist(import_run:, uploaded_file: upload, service_name: :s3)

    expect { store.cleanup(import_run) }.to raise_error(NhsDmd::ArchiveStore::Error, 'archive_not_terminal')
    import_run.update!(status: :completed)
    key = import_run.archive_key

    store.cleanup(import_run)

    expect(services.fetch(:s3)).not_to exist(key)
    expect(import_run.reload).to have_attributes(
      archive_service_name: nil,
      archive_key: nil,
      archive_checksum: nil,
      archive_byte_size: nil
    )
  end

  it 'converts a live legacy path and removes it only after verified persistence' do
    legacy = Tempfile.create(['legacy-release', '.zip']).tap do |file|
      file.write(payload)
      file.close
    end
    import_run.update!(archive_path: legacy.path)

    store.convert_legacy(import_run:, service_name: :s3)

    expect(import_run.reload.archive_service_name).to eq('s3')
    expect(import_run.archive_path).to be_nil
    expect(File).not_to exist(legacy.path)
  ensure
    legacy&.close
    FileUtils.rm_f(legacy.path) if legacy
  end

  it 'returns a PHI-safe failure and no durable reference when verification fails' do
    service = services.fetch(:persistent)
    allow(service).to receive(:open).and_raise(ActiveStorage::IntegrityError)

    expect do
      store.persist(import_run:, uploaded_file: upload, service_name: :persistent)
    end.to raise_error(NhsDmd::ArchiveStore::Error, 'archive_persistence_failed')

    expect(import_run.reload.archive_key).to be_nil
    expect(import_run.error_message).to be_nil
  end

  it 'does not expose a public archive URL boundary' do
    expect(store).not_to respond_to(:url)
  end

  def persistent_root
    @persistent_root ||= Dir.mktmpdir('nhs-dmd-persistent')
  end

  def s3_root
    @s3_root ||= Dir.mktmpdir('nhs-dmd-s3')
  end

  def payload
    'verified archive data'
  end
end
