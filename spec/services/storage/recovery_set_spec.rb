# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Storage::RecoverySet do
  it 'requires only database and Disk references for Disk-only live identities' do
    set = described_class.new(blob_scope: blob_scope('persistent'))

    expect(set.required_backends).to eq(%i[disk])
    expect(set.record(database_reference: 'db-1', disk_reference: 'disk-1').to_h).to eq(
      database_reference: 'db-1',
      storage_references: { disk: 'disk-1' },
      migration_phase: nil
    )
  end

  it 'requires only database and S3 references for S3-only live identities' do
    set = described_class.new(blob_scope: blob_scope('s3'))

    expect(set.required_backends).to eq(%i[s3])
    expect(set.record(database_reference: 'db-1', s3_reference: 's3-1').storage_references)
      .to eq(s3: 's3-1')
  end

  it 'requires both storage references for mirror identities or an active migration' do
    mirror_set = described_class.new(blob_scope: blob_scope('persistent_with_s3_mirror'))
    active_run = instance_double(StorageMigrationRun, finalized?: false, phase: 'rollback_window')
    migration_set = described_class.new(blob_scope: blob_scope('persistent'), migration_run: active_run)

    expect(mirror_set.required_backends).to eq(%i[disk s3])
    expect(migration_set.required_backends).to eq(%i[disk s3])
  end

  it 'rejects missing recovery references with a PHI-safe code' do
    set = described_class.new(blob_scope: blob_scope('s3'))

    expect { set.record(database_reference: 'db-1') }
      .to raise_error(described_class::Error, 's3_recovery_reference_required')
  end

  def blob_scope(*service_names)
    instance_double(ActiveRecord::Relation, distinct: instance_double(ActiveRecord::Relation, pluck: service_names))
  end
end
