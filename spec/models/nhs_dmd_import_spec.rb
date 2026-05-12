# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmdImport do
  describe '#start!' do
    it 'sets started_at to current time when blank' do
      import = described_class.create!(uploaded_filename: 'release.zip')

      freeze_time do
        expect { import.start! }.to change { import.reload.started_at }.from(nil).to(Time.current)
      end
    end

    it 'does not update started_at when already present' do
      existing_time = 1.day.ago.round
      import = described_class.create!(uploaded_filename: 'release.zip', started_at: existing_time)

      expect { import.start! }.not_to change { import.reload.started_at }
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
end
