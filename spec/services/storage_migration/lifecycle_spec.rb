# frozen_string_literal: true

require 'rails_helper'

RSpec.describe StorageMigration::Lifecycle do
  before do
    allow(ActiveStorage::Blob).to receive(:services).and_return(service_registry)
    blobs.zip(%w[first second]).each do |blob, payload|
      service_registry.fetch(source_service_name).upload(blob.key, StringIO.new(payload), checksum: blob.checksum)
      service_registry.fetch(destination_service_name).upload(blob.key, StringIO.new(payload), checksum: blob.checksum)
    end
  end

  after do
    ActiveStorage::Blob.where(id: blobs.map(&:id)).delete_all
    FileUtils.rm_rf(source_root)
    FileUtils.rm_rf(destination_root)
  end

  it 'reconciles a stable set only after storage mutations stop and mirror work drains' do
    blocked = lifecycle.reconcile(apply: true, mutations_quiesced: false, mirror_queue_drained: true)

    expect(blocked.to_h).to include(eligible: false, applied: false, reason: :mutations_not_quiesced)
    expect(run.reload).to be_backfill

    result = lifecycle.reconcile(apply: true, mutations_quiesced: true, mirror_queue_drained: true)

    expect(result.to_h).to include(eligible: true, applied: true, reason: :reconciled)
    expect(run.reload).to be_reconciled
    expect(run.stable_blob_count).to eq(2)
  end

  it 'blocks reconciliation and cutover while mirror work is pending' do
    result = lifecycle.reconcile(apply: true, mutations_quiesced: true, mirror_queue_drained: false)

    expect(result.to_h).to include(eligible: false, applied: false, reason: :mirror_queue_pending)
    expect(lifecycle.cutover(apply: true).reason).to eq(:reconciliation_required)
    expect(blobs.map { it.reload.service_name }).to all(eq('persistent'))
  end

  it 'blocks cutover when a destination blob is corrupt' do
    service_registry.fetch(destination_service_name).upload(
      blobs.first.key,
      StringIO.new('corrupt'),
      checksum: Digest::MD5.base64digest('corrupt')
    )

    result = lifecycle.reconcile(apply: true, mutations_quiesced: true, mirror_queue_drained: true)

    expect(result.to_h).to include(eligible: false, applied: false, reason: :destination_unverified)
    expect(lifecycle.cutover(apply: true).reason).to eq(:reconciliation_required)
    expect(blobs.map { it.reload.service_name }).to all(eq('persistent'))
  end

  it 'keeps cutover dry-run by default' do
    reconcile!

    result = lifecycle.cutover

    expect(result.to_h).to include(eligible: true, applied: false, reason: :cutover_ready)
    expect(blobs.map { it.reload.service_name }).to all(eq('persistent'))
  end

  it 'keeps reconciliation dry-run by default and does not repair missing objects' do
    destination = service_registry.fetch(destination_service_name)
    destination.delete(blobs.first.key)

    result = lifecycle.reconcile(mutations_quiesced: true, mirror_queue_drained: true)

    expect(result.to_h).to include(eligible: false, applied: false, reason: :destination_unverified)
    expect(destination).not_to exist(blobs.first.key)
    expect(run.reload).to be_backfill
  end

  it 'atomically enters the destination-primary rollback window' do
    reconcile!

    result = lifecycle.cutover(apply: true, rollback_window: 2.hours)

    expect(result.to_h).to include(eligible: true, applied: true, reason: :cutover_complete)
    expect(blobs.map { it.reload.service_name }).to all(eq('s3_with_persistent_mirror'))
    expect(run.reload).to be_rollback_window
    expect(run.rollback_deadline).to eq(clock.call + 2.hours)
  end

  it 'supports the same cutover transition from S3 to Disk' do
    run.update!(source_service_name: :s3, destination_service_name: :persistent)
    blobs.each { it.update!(service_name: 's3') }
    reconcile!

    lifecycle.cutover(apply: true)

    expect(blobs.map { it.reload.service_name }).to all(eq('persistent_with_s3_mirror'))
  end

  it 'rolls every service identity back when the cutover transaction fails' do
    reconcile!
    failing_lifecycle = described_class.new(
      run:,
      blob_scope:,
      service_registry:,
      owner_role: -> { true },
      clock:,
      after_transition: -> { raise ActiveRecord::Rollback }
    )

    result = failing_lifecycle.cutover(apply: true)

    expect(result.to_h).to include(applied: false, reason: :transaction_rolled_back)
    expect(blobs.map { it.reload.service_name }).to all(eq('persistent'))
    expect(run.reload).to be_reconciled
  end

  it 'rolls back during the window after the source is verified and mirror work drains' do
    reconcile!
    lifecycle.cutover(apply: true, rollback_window: 2.hours)

    result = lifecycle.rollback(apply: true, source_verified: true, mirror_queue_drained: true)

    expect(result.to_h).to include(eligible: true, applied: true, reason: :rollback_complete)
    expect(blobs.map { it.reload.service_name }).to all(eq('persistent_with_s3_mirror'))
    expect(run.reload).to be_backfill
  end

  it 'keeps rollback dry-run by default' do
    reconcile!
    lifecycle.cutover(apply: true, rollback_window: 2.hours)

    result = lifecycle.rollback(source_verified: true, mirror_queue_drained: true)

    expect(result.to_h).to include(eligible: true, applied: false, reason: :rollback_ready)
    expect(blobs.map { it.reload.service_name }).to all(eq('s3_with_persistent_mirror'))
  end

  it 'rejects rollback after the rollback window' do
    reconcile!
    lifecycle.cutover(apply: true, rollback_window: 2.hours)
    late_lifecycle = described_class.new(
      run:,
      blob_scope:,
      service_registry:,
      owner_role: -> { true },
      clock: -> { clock.call + 3.hours }
    )

    result = late_lifecycle.rollback(apply: true, source_verified: true, mirror_queue_drained: true)

    expect(result.to_h).to include(eligible: false, applied: false, reason: :rollback_window_expired)
    expect(blobs.map { it.reload.service_name }).to all(eq('s3_with_persistent_mirror'))
  end

  it 'finalizes only after the window, acceptance, recovery, and final reconciliation' do
    reconcile!
    lifecycle.cutover(apply: true, rollback_window: 2.hours)
    late_lifecycle = lifecycle_at(clock.call + 3.hours)

    blocked = late_lifecycle.finalize(apply: true, acceptance_verified: false,
                                      recovery_verified: true, final_reconciled: true)
    result = late_lifecycle.finalize(apply: true, acceptance_verified: true,
                                     recovery_verified: true, final_reconciled: true)

    expect(blocked.to_h).to include(eligible: false, applied: false, reason: :acceptance_required)
    expect(result.to_h).to include(eligible: true, applied: true, reason: :finalized)
    expect(blobs.map { it.reload.service_name }).to all(eq('s3'))
    expect(run.reload).to be_finalized
  end

  it 'keeps finalization dry-run by default' do
    reconcile!
    lifecycle.cutover(apply: true, rollback_window: 1.second)
    late_lifecycle = described_class.new(
      run:,
      blob_scope:,
      service_registry:,
      owner_role: -> { true },
      clock: -> { clock.call + 2.seconds }
    )

    result = late_lifecycle.finalize(acceptance_verified: true,
                                     recovery_verified: true, final_reconciled: true)

    expect(result.to_h).to include(eligible: true, applied: false, reason: :finalization_ready)
    expect(blobs.map { it.reload.service_name }).to all(eq('s3_with_persistent_mirror'))
  end

  it 'requires finalization and no live source dependency before source retirement' do
    expect(lifecycle.retirement_eligibility(live_source_dependency: false).reason).to eq(:finalization_required)
    reconcile!
    lifecycle.cutover(apply: true, rollback_window: 1.second)
    late_lifecycle = lifecycle_at(clock.call + 2.seconds)
    late_lifecycle.finalize(apply: true, acceptance_verified: true,
                            recovery_verified: true, final_reconciled: true)

    expect(late_lifecycle.retirement_eligibility(live_source_dependency: true).reason)
      .to eq(:live_source_dependency)
    expect(late_lifecycle.retirement_eligibility(live_source_dependency: false).to_h)
      .to include(eligible: true, applied: false, reason: :source_retirement_eligible)
  end

  def reconcile!
    lifecycle.reconcile(apply: true, mutations_quiesced: true, mirror_queue_drained: true)
  end

  def clock
    @clock ||= -> { Time.zone.parse('2026-08-02 08:00:00') }
  end

  def source_service_name
    :persistent
  end

  def destination_service_name
    :s3
  end

  def source_root
    @source_root ||= Dir.mktmpdir('storage-lifecycle-source')
  end

  def destination_root
    @destination_root ||= Dir.mktmpdir('storage-lifecycle-destination')
  end

  def service_registry
    @service_registry ||= begin
      persistent = ActiveStorage::Service::DiskService.new(root: source_root)
      s3 = ActiveStorage::Service::DiskService.new(root: destination_root)
      {
        persistent:, s3:, 'persistent' => persistent, 's3' => s3,
        'persistent_with_s3_mirror' => persistent, 's3_with_persistent_mirror' => s3
      }
    end
  end

  def blobs
    @blobs ||= %w[first second].map { |payload| create_blob(payload:) }
  end

  def blob_scope
    @blob_scope ||= ActiveStorage::Blob.where(id: blobs)
  end

  def run
    @run ||= StorageMigrationRun.create!(source_service_name:, destination_service_name:)
  end

  def lifecycle
    @lifecycle ||= lifecycle_at(clock.call)
  end

  def lifecycle_at(time)
    described_class.new(run:, blob_scope:, service_registry:, owner_role: -> { true }, clock: -> { time })
  end

  def create_blob(payload:)
    blob = ActiveStorage::Blob.new(
      key: SecureRandom.base58(28),
      filename: 'opaque.bin',
      content_type: 'application/octet-stream',
      byte_size: payload.bytesize,
      checksum: Digest::MD5.base64digest(payload),
      service_name: source_service_name
    )
    blob.save!(validate: false)
    blob
  end
end
