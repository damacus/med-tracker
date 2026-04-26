# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmdImport do
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
