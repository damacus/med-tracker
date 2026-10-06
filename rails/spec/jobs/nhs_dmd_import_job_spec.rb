# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmdImportJob do
  let(:import_run) do
    NhsDmdImport.create!(
      uploaded_filename: 'nhsbsa_dmd_release.zip',
      archive_service_name: 'persistent',
      archive_key: SecureRandom.uuid,
      archive_checksum: Digest::MD5.base64digest('archive'),
      archive_byte_size: 7,
      status: :queued
    )
  end

  let(:service) { instance_double(NhsDmd::ReleaseArchiveImport) }
  let(:archive_store) { instance_double(NhsDmd::ArchiveStore, cleanup: nil) }

  before do
    allow(NhsDmd::ReleaseArchiveImport).to receive(:new).and_return(service)
    allow(NhsDmd::ArchiveStore).to receive(:new).and_return(archive_store)
    allow(archive_store).to receive(:open).with(import_run).and_yield('/private/tmp/opaque-release')
  end

  def progress_update(message, **overrides)
    {
      status: :importing,
      message: message,
      total_records: 1200,
      processed_records: nil,
      imported_count: 0,
      skipped_count: 0
    }.merge(overrides).compact
  end

  def complete_import_via(service)
    allow(service).to receive(:import) do |_path, progress_callback:|
      completed_progress_updates.each { |progress| progress_callback.call(progress) }
      completed_import_result
    end
  end

  def completed_progress_updates
    [
      progress_update('Extracting release archive', status: :extracting),
      progress_update('Counted 270 AMPP records and 930 GTIN records', status: :counting),
      progress_update('Starting AMPP name import', processed_records: 0),
      progress_update('Starting GTIN import', processed_records: 270),
      progress_update('Processed 880 import records', processed_records: 880, imported_count: 480, skipped_count: 20)
    ]
  end

  def completed_import_result
    NhsDmd::ReleaseImport::Result.new(
      created_count: 700,
      updated_count: 200,
      unchanged_count: 50,
      skipped_expired_count: 10,
      skipped_missing_name_count: 15,
      skipped_invalid_count: 5
    )
  end

  def completed_import_attributes
    {
      status: 'completed',
      total_records: 1200,
      processed_records: 1200,
      imported_count: 900,
      skipped_count: 30,
      created_count: 700,
      updated_count: 200,
      unchanged_count: 50,
      skipped_expired_count: 10,
      skipped_missing_name_count: 15,
      skipped_invalid_count: 5
    }
  end

  it 'marks the import as completed and captures progress updates' do
    complete_import_via(service)

    described_class.perform_now(import_run.id)

    import_run.reload
    expect(import_run).to have_attributes(completed_import_attributes)
    expect(import_run.log).to include('Starting AMPP name import')
      .and include('Starting GTIN import')
      .and include('Processed 880 import records')
    expect(service).to have_received(:import).with('/private/tmp/opaque-release', progress_callback: anything)
    expect(archive_store).to have_received(:cleanup).with(import_run)
  end

  it 'keeps a completed import successful when archive cleanup fails' do
    complete_import_via(service)
    allow(archive_store).to receive(:cleanup)
      .and_raise(NhsDmd::ArchiveStore::Error, 'archive_cleanup_failed')
    allow(Observability::DiagnosticEvent).to receive(:emit)

    expect do
      described_class.perform_now(import_run.id)
    end.not_to raise_error

    expect(import_run.reload).to have_attributes(completed_import_attributes)
    expect(Observability::DiagnosticEvent).to have_received(:emit).with(
      component: :nhs_dmd_import,
      reason: :operation_failed,
      severity: :error,
      error: an_instance_of(NhsDmd::ArchiveStore::Error)
    )
  end

  it 'marks the import as failed when the archive import errors' do
    allow(service).to receive(:import).and_raise(NhsDmd::ReleaseArchiveImport::Error, 'bad zip')

    expect do
      described_class.perform_now(import_run.id)
    end.not_to raise_error

    import_run.reload
    expect(import_run).to be_failed
    expect(import_run.error_message).to eq('bad zip')
    expect(import_run.completed_at).to be_present
    expect(archive_store).to have_received(:cleanup).with(import_run)
  end
end
