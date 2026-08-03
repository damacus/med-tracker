# frozen_string_literal: true

require 'rails_helper'

RSpec.describe StorageMigration::Command do
  let(:output) { StringIO.new }
  let(:error) { StringIO.new }
  let(:environment) do
    {
      'STORAGE_MIGRATION_ACTION' => 'start',
      'STORAGE_MIGRATION_SOURCE' => 'persistent',
      'STORAGE_MIGRATION_DESTINATION' => 's3',
      'STORAGE_MIGRATION_PHASE' => 'persistent_with_s3_mirror'
    }
  end
  let(:owner_role) { -> { true } }
  let(:command) { described_class.new(environment:, output:, error:, owner_role:) }

  def backfill_result
    @backfill_result ||= StorageMigration::Backfill::Result.new(
      run_id: SecureRandom.uuid,
      source_service_name: :persistent,
      destination_service_name: :s3,
      phase: :persistent_with_s3_mirror,
      processed_count: 3,
      verified_count: 3,
      failed_count: 0
    )
  end

  before do
    allow(StorageMigration::Backfill).to receive(:new)
      .and_return(instance_double(StorageMigration::Backfill, call: backfill_result))
  end

  it 'defaults migration start to a read-only JSON dry run' do
    expect { expect(command.call).to eq(0) }.not_to change(StorageMigrationRun, :count)

    expect(StorageMigration::Backfill).to have_received(:new).with(hash_including(copy_missing: false))
    expect(JSON.parse(output.string)).to include(
      'outcome' => 'passed',
      'action' => 'start',
      'applied' => false,
      'processed_count' => 3,
      'verified_count' => 3,
      'failed_count' => 0
    )
  end

  it 'requires an exact confirmation before applying a mutation' do
    environment['STORAGE_MIGRATION_APPLY'] = 'true'

    expect { expect(command.call).to eq(2) }.not_to change(StorageMigrationRun, :count)
    expect(JSON.parse(error.string)).to include(
      'outcome' => 'failed',
      'failure_code' => 'confirmation_required'
    )
  end

  it 'creates an opaque resumable run only when start is explicitly applied' do
    environment.merge!(
      'STORAGE_MIGRATION_APPLY' => 'true',
      'STORAGE_MIGRATION_CONFIRM' => 'persistent-to-s3'
    )

    expect { expect(command.call).to eq(0) }.to change(StorageMigrationRun, :count).by(1)

    expect(StorageMigration::Backfill).to have_received(:new).with(hash_including(copy_missing: true))
    expect(JSON.parse(output.string).fetch('run_id'))
      .to match(Observability::CorrelationContext::UUID_PATTERN)
  end

  it 'rejects unsupported directions and phase mismatches without loading storage services' do
    environment['STORAGE_MIGRATION_DESTINATION'] = 'persistent'

    expect(command.call).to eq(2)
    expect(StorageMigration::Backfill).not_to have_received(:new)
    expect(JSON.parse(error.string)).to include('failure_code' => 'invalid_storage_direction')
  end

  it 'enforces the owner-capable database role before reading migration state' do
    command = described_class.new(environment:, output:, error:, owner_role: -> { false })

    expect(command.call).to eq(3)
    expect(JSON.parse(error.string)).to include('failure_code' => 'owner_role_required')
  end

  it 'dispatches resume and every lifecycle operation with explicit gates' do
    run = StorageMigrationRun.create!(source_service_name: :persistent, destination_service_name: :s3)
    lifecycle = stub_lifecycle(run)

    dispatch_actions(run)
    expect_lifecycle_dispatches(run, lifecycle)
  end

  def stub_lifecycle(run)
    result = lifecycle_result(run)
    lifecycle = instance_double(
      StorageMigration::Lifecycle,
      reconcile: result,
      cutover: result,
      rollback: result,
      finalize: result,
      retirement_eligibility: result
    )
    allow(StorageMigration::Lifecycle).to receive(:new).and_return(lifecycle)
    lifecycle
  end

  def lifecycle_result(run)
    StorageMigration::Lifecycle::Result.new(
      run_id: run.run_id,
      eligible: true,
      applied: false,
      reason: :ready,
      processed_count: 2,
      verified_count: 2,
      failed_count: 0
    )
  end

  def dispatch_actions(run)
    migration_actions.each do |action|
      reset_output
      environment.merge!(migration_environment(action, run))
      expect_successful_dispatch(action)
    end
  end

  def expect_successful_dispatch(action)
    expect(command.call).to eq(0)
    expect(JSON.parse(output.string)).to include('action' => action)
  end

  def expect_lifecycle_dispatches(run, lifecycle)
    expect(StorageMigration::Backfill).to have_received(:new).with(hash_including(run_id: run.run_id))
    expect(lifecycle).to have_received(:cutover).twice
    %i[reconcile rollback finalize retirement_eligibility].each do |operation|
      expect(lifecycle).to have_received(operation)
    end
  end

  def migration_actions
    %w[resume reconcile cutover_eligibility cutover rollback finalize retirement_eligibility]
  end

  def reset_output
    output.truncate(0)
    output.rewind
  end

  def migration_environment(action, run)
    {
      'STORAGE_MIGRATION_ACTION' => action,
      'STORAGE_MIGRATION_RUN_ID' => run.run_id,
      'STORAGE_MIGRATION_PHASE' => command_phase(action),
      'STORAGE_MIGRATION_MUTATIONS_QUIESCED' => 'true',
      'STORAGE_MIGRATION_MIRROR_QUEUE_DRAINED' => 'true',
      'STORAGE_MIGRATION_SOURCE_VERIFIED' => 'true',
      'STORAGE_MIGRATION_ACCEPTANCE_VERIFIED' => 'true',
      'STORAGE_MIGRATION_RECOVERY_VERIFIED' => 'true',
      'STORAGE_MIGRATION_FINAL_RECONCILED' => 'true',
      'STORAGE_MIGRATION_LIVE_SOURCE_DEPENDENCY' => 'false'
    }
  end

  it 'never writes configured secrets or blob details to command output' do
    environment.merge!(
      'ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY' => 'do-not-print',
      'ORIGINAL_FILENAME' => 'private-medication-record.zip'
    )

    command.call

    expect(output.string).not_to include('do-not-print', 'private-medication-record.zip')
  end

  def command_phase(action)
    return 's3_with_persistent_mirror' if %w[rollback finalize].include?(action)
    return 's3' if action == 'retirement_eligibility'

    'persistent_with_s3_mirror'
  end
end
