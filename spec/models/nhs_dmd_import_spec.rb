# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmdImport do
  describe 'durable archive reference' do
    it 'accepts a complete service-neutral reference' do
      import = described_class.new(
        uploaded_filename: 'release.zip',
        archive_service_name: 'persistent',
        archive_key: SecureRandom.uuid,
        archive_checksum: Digest::MD5.base64digest('archive'),
        archive_byte_size: 7
      )

      expect(import).to be_valid
    end

    it 'rejects partial service-neutral references' do
      import = described_class.new(uploaded_filename: 'release.zip', archive_service_name: 'persistent')

      expect(import).not_to be_valid
      expect(import.errors[:archive_key]).to be_present
    end

    it 'retains bounded compatibility with an existing archive path' do
      import = described_class.new(uploaded_filename: 'release.zip', archive_path: '/legacy/release.zip')

      expect(import).to be_valid
      expect(import).to be_legacy_archive
    end

    it 'filters archive keys and checksums from framework parameter logging' do
      filtered = ActiveSupport::ParameterFilter
                 .new(Rails.application.config.filter_parameters)
                 .filter(archive_key: 'opaque-key', archive_checksum: 'private-checksum')

      expect(filtered).to eq(archive_key: '[FILTERED]', archive_checksum: '[FILTERED]')
    end
  end

  describe '#start!' do
    it 'sets started_at to current time when blank' do
      import = described_class.create!(uploaded_filename: 'release.zip')

      freeze_time do
        expect { import.start! }.to(change { import.reload.started_at }.from(nil).to(Time.current))
      end
    end

    it 'does not update started_at when already present' do
      existing_time = 1.day.ago.round
      import = described_class.create!(uploaded_filename: 'release.zip', started_at: existing_time)

      expect { import.start! }.not_to(change { import.reload.started_at })
    end
  end

  describe '#complete!' do
    it 'includes unchanged records in the fallback processed total' do
      import = described_class.create!(uploaded_filename: 'nhsbsa_dmd_release.zip')
      result = NhsDmd::ReleaseImport::Result.new(
        created_count: 10,
        updated_count: 5,
        unchanged_count: 20,
        skipped_expired_count: 2,
        skipped_missing_name_count: 3,
        skipped_invalid_count: 4
      )

      import.complete!(result)

      expect(import.reload).to have_attributes(
        status: 'completed',
        processed_records: 44,
        unchanged_count: 20
      )
    end
  end

  describe 'active import invariant' do
    it 'permits only one queued, extracting, counting, or importing record' do
      described_class.create!(uploaded_filename: 'queued.zip', status: :queued)

      expect do
        described_class.create!(uploaded_filename: 'importing.zip', status: :importing)
      end.to raise_error(ActiveRecord::RecordNotUnique)
    end

    it 'allows a new active import after the previous import has completed' do
      import = described_class.create!(uploaded_filename: 'queued.zip', status: :queued)
      import.update!(status: :completed, completed_at: Time.current)

      expect do
        described_class.create!(uploaded_filename: 'next.zip', status: :queued)
      end.not_to raise_error
    end
  end

  describe 'Turbo refresh broadcasts' do
    it 'broadcasts after committed progress updates' do
      import = described_class.create!(uploaded_filename: 'release.zip')
      allow(import).to receive(:broadcast_refresh_to)

      import.apply_progress!(status: :importing, message: 'Importing records')

      expect(import).to have_received(:broadcast_refresh_to).with(import)
    end

    it 'broadcasts after committed terminal updates' do
      import = described_class.create!(uploaded_filename: 'release.zip')
      allow(import).to receive(:broadcast_refresh_to)
      result = NhsDmd::ReleaseImport::Result.new(
        created_count: 1,
        updated_count: 0,
        unchanged_count: 0,
        skipped_expired_count: 0,
        skipped_missing_name_count: 0,
        skipped_invalid_count: 0
      )

      import.complete!(result)

      expect(import).to have_received(:broadcast_refresh_to).with(import)
    end
  end

  describe '#progress_percentage' do
    subject(:import) { described_class.new }

    context 'when total_records is nil' do
      it 'returns 0' do
        import.total_records = nil
        expect(import.progress_percentage).to eq(0)
      end
    end

    context 'when total_records is 0' do
      it 'returns 0' do
        import.total_records = 0
        expect(import.progress_percentage).to eq(0)
      end
    end

    context 'when total_records is negative' do
      it 'returns 0' do
        import.total_records = -5
        expect(import.progress_percentage).to eq(0)
      end
    end

    context 'when total_records is positive' do
      it 'calculates the percentage correctly' do
        import.total_records = 100
        import.processed_records = 50
        expect(import.progress_percentage).to eq(50)
      end

      it 'floors the calculated percentage' do
        import.total_records = 3
        import.processed_records = 1
        expect(import.progress_percentage).to eq(33)
      end
    end
  end
end
