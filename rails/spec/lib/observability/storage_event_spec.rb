# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::StorageEvent do
  before do
    allow(Observability::CanonicalLogger).to receive(:write)
  end

  it 'emits the selected steady storage mode' do
    event = described_class.configuration(service: :persistent)

    expect(event.to_h).to include(
      'event.name' => 'storage.stage',
      'event.outcome' => 'success',
      'medtracker.reason' => 'configured',
      'medtracker.storage.operation' => 'configuration',
      'medtracker.storage.backend' => 'disk',
      'medtracker.storage.phase' => 'persistent'
    )
  end

  it 'emits aggregate migration progress for either direction' do
    event = described_class.migration(
      service: :s3_with_persistent_mirror,
      outcome: :success,
      reason: :completed,
      counts: { processed_count: 12, verified_count: 11, failed_count: 1 }
    )

    expect(event.to_h).to include(
      'event.name' => 'storage.stage',
      'medtracker.storage.operation' => 'migration',
      'medtracker.storage.backend' => 'dual',
      'medtracker.storage.phase' => 's3_with_persistent_mirror',
      'medtracker.storage.processed_count' => 12,
      'medtracker.storage.verified_count' => 11,
      'medtracker.storage.failed_count' => 1
    )
  end
end
