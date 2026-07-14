# frozen_string_literal: true

require 'rails_helper'

module HostedRestoreSpec
  class RecordingRunner
    attr_reader :command_ids

    def initialize(outputs, failure)
      @outputs = outputs
      @failure = failure
      @command_ids = []
    end

    def call(command)
      command_ids << command.fetch(:id)
      return failed_result if command.fetch(:id) == @failure

      HostedRestore::CommandResult.new(exit_code: 0, output: @outputs.fetch(command.fetch(:id)))
    end

    private

    def failed_result
      output = { outcome: 'failed', failure_code: 'verification_failed' }
      HostedRestore::CommandResult.new(exit_code: 1, output:)
    end
  end
end

RSpec.describe HostedRestore::Rehearsal do
  let(:durable_root) { Rails.root.join('storage/spec-hosted-restore-evidence') }
  let(:output) { durable_root.join('rehearsal-2026-q3') }
  let(:environment) do
    {
      'DATABASE_BACKUP_ID' => 'database-snapshot-2026-07-14T010000Z',
      'ATTACHMENT_BACKUP_ID' => 'attachments-snapshot-2026-07-14T010000Z',
      'RESTORE_TARGET_ID' => 'isolated-restore-2026-q3',
      'APP_IMAGE' => 'ghcr.io/damacus/med-tracker:v0.5.0-rc1',
      'RUNTIME_APP_IMAGE' => 'ghcr.io/damacus/med-tracker:v0.5.0-rc1',
      'TESTER' => 'restore-operator',
      'HOUSEHOLD_A_ID' => '910001',
      'HOUSEHOLD_B_ID' => '920002',
      'WORM_REFERENCE' => 'object-lock-checkpoint-2026-07-14',
      'WORM_HEADS_JSON' => JSON.generate(
        sample_a: { chain_epoch: SecureRandom.uuid, sequence: 11, entry_hash: 'a' * 64 },
        sample_b: { chain_epoch: SecureRandom.uuid, sequence: 22, entry_hash: 'b' * 64 }
      ),
      'EVIDENCE_ROOT' => durable_root.to_s,
      'EVIDENCE_OUTPUT' => output.to_s
    }
  end
  let(:success_outputs) do
    {
      'owner.db_migrate' => {
        outcome: 'passed', schema_version: '20260714090000', database_role: 'med_tracker_owner'
      },
      'runtime.restore_verify' => {
        outcome: 'passed', schema_version: '20260714090000', database_role: 'med_tracker_app',
        app_image: 'ghcr.io/damacus/med-tracker:v0.5.0-rc1',
        forced_rls: true, default_deny: true, isolation: { clinical: true, audit: true, attachments: true },
        storage: { samples_verified: 2 }
      },
      'audit.combined_verify' => {
        outcome: 'passed', scope: 'combined', samples_verified: 2, checked_entries: 8,
        checked_checkpoints: 2, checked_objects: 10, verified_heads: 2, worm_comparison: 'match'
      }
    }
  end

  before do
    FileUtils.rm_rf(durable_root)
    FileUtils.mkdir_p(durable_root)
  end

  after { FileUtils.rm_rf(durable_root) }

  it 'runs owner migration before forced-RLS and combined audit verification' do
    runner = recording_runner(success_outputs)

    result = described_class.new(environment:, runner:, clock: -> { Time.utc(2026, 7, 14, 9) }).call

    expect(result).to eq(0)
    expect(runner.command_ids).to eq(%w[owner.db_migrate runtime.restore_verify audit.combined_verify])
    evidence = JSON.parse(output.join('evidence.json').read)
    expect(evidence.fetch('outcome')).to eq('passed')
    expect(evidence.fetch('commands').pluck('description')).to eq(
      [
        'env DATABASE_ROLE=med_tracker_owner rails hosted_restore:migrate',
        'env DATABASE_ROLE=med_tracker_app rails hosted_restore:verify_runtime',
        'env DATABASE_ROLE=med_tracker_audit_verifier rails hosted_restore:verify_audit'
      ]
    )
  end

  it 'writes sanitized machine-readable and human-readable evidence without tenant ids or paths' do
    described_class.new(environment:, runner: recording_runner(success_outputs)).call

    json = output.join('evidence.json').read
    parsed = JSON.parse(json)
    markdown = output.join('evidence.md').read

    expect(json).to include('database-snapshot-2026-07-14T010000Z', 'owner.db_migrate', 'worm_comparison')
    expect(markdown).to include('# Hosted restore rehearsal evidence', 'Final outcome: PASS')
    expect(parsed.to_s).not_to include('910001', '920002', output.to_s, 'WORM_HEADS_JSON')
    expect(markdown).not_to include('910001', '920002', output.to_s)
  end

  it 'stops after a failed stage and records failure without claiming success' do
    runner = recording_runner(success_outputs, failure: 'runtime.restore_verify')

    result = described_class.new(environment:, runner:).call
    evidence = JSON.parse(output.join('evidence.json').read)

    expect(result).to eq(1)
    expect(runner.command_ids).to eq(%w[owner.db_migrate runtime.restore_verify])
    expect(evidence).to include('outcome' => 'failed')
    expect(evidence.fetch('failures').sole).to include(
      'command_id' => 'runtime.restore_verify',
      'remediation' => 'correct the isolated restore or configuration and repeat the same rehearsal'
    )
  end

  it 'requires exact passing stage schemas and matching owner/runtime versions and image' do
    invalid_stage_outputs.each_with_index { |stage_outputs, index| expect_invalid_stage(stage_outputs, index) }
  end

  it 'refuses missing, placeholder, or ambiguous inputs before running commands' do
    expect_inputs_rejected(invalid_input_environments)
  end

  it 'refuses to overwrite existing evidence' do
    FileUtils.mkdir_p(output)
    expect do
      described_class.new(environment:, runner: recording_runner(success_outputs)).call
    end.to raise_error(HostedRestore::EvidenceWriter::AlreadyExists)
  end

  it 'requires a durable-root-contained output and rejects temporary and symlink escapes' do
    outside = prepare_symlink_escape
    expect_invalid_locations(unsafe_evidence_environments(outside))
  ensure
    FileUtils.rm_rf(outside)
  end

  it 'parses only whitelisted structured failure JSON from stderr' do
    status = instance_double(Process::Status, exitstatus: 1)
    capture = lambda do |*_arguments|
      ['', "tenant=secret-person-name\n{\"outcome\":\"failed\",\"failure_code\":\"runtime_role_required\"}\n", status]
    end
    result = HostedRestore::CommandRunner.new(capture:).call(HostedRestore::Rehearsal::COMMANDS.second)

    expect(result).to have_attributes(
      exit_code: 1,
      output: { outcome: 'failed', failure_code: 'runtime_role_required' }
    )
    expect(result.output.to_s).not_to include('secret-person-name')
  end

  def recording_runner(outputs, failure: nil)
    HostedRestoreSpec::RecordingRunner.new(outputs, failure)
  end

  def invalid_stage_outputs
    [
      invalid_owner_stage,
      invalid_runtime_stage(success_outputs.fetch('runtime.restore_verify').except(:storage)),
      invalid_runtime_stage(schema_version: '1'),
      invalid_runtime_stage(app_image: 'other:v1'),
      invalid_audit_stage
    ]
  end

  def expect_invalid_stage(stage_outputs, index)
    result, evidence = run_invalid_stage(stage_outputs, index)
    expect(result).to eq(1)
    expect(evidence).to include('outcome' => 'failed')
    expect(evidence.fetch('failures').last.fetch('failure_code')).to eq('invalid_stage_output')
  end

  def run_invalid_stage(stage_outputs, index)
    invalid_environment = environment.merge('EVIDENCE_OUTPUT' => durable_root.join("invalid-#{index}").to_s)
    result = described_class.new(environment: invalid_environment, runner: recording_runner(stage_outputs)).call
    evidence_path = Pathname.new(invalid_environment.fetch('EVIDENCE_OUTPUT')).join('evidence.json')
    [result, JSON.parse(evidence_path.read)]
  end

  def invalid_owner_stage
    success_outputs.merge('owner.db_migrate' => { outcome: 'ok' })
  end

  def invalid_runtime_stage(replacement = nil, **changes)
    runtime = replacement || success_outputs.fetch('runtime.restore_verify').merge(changes)
    success_outputs.merge('runtime.restore_verify' => runtime)
  end

  def invalid_audit_stage
    audit = success_outputs.fetch('audit.combined_verify').merge(verified_heads: 1)
    success_outputs.merge('audit.combined_verify' => audit)
  end

  def invalid_input_environments
    [
      environment.except('DATABASE_BACKUP_ID'),
      environment.merge('APP_IMAGE' => 'ghcr.io/damacus/med-tracker:latest'),
      environment.merge('RUNTIME_APP_IMAGE' => 'ghcr.io/damacus/med-tracker:v0.5.0-rc2'),
      environment.merge('HOUSEHOLD_A_ID' => '910001junk'),
      environment.merge('HOUSEHOLD_B_ID' => environment.fetch('HOUSEHOLD_A_ID')),
      environment.merge('WORM_HEADS_JSON' => invalid_worm_heads)
    ]
  end

  def invalid_worm_heads
    JSON.generate(
      sample_a: { chain_epoch: 'not-a-uuid', sequence: '11', entry_hash: 'a' * 64 },
      sample_b: { chain_epoch: SecureRandom.uuid, sequence: 22, entry_hash: 'b' * 64 },
      extra: {}
    )
  end

  def expect_inputs_rejected(invalid_inputs)
    invalid_inputs.each do |invalid|
      runner = recording_runner(success_outputs)
      expect { described_class.new(environment: invalid, runner:).call }
        .to raise_error(HostedRestore::Input::Invalid)
      expect(runner.command_ids).to be_empty
    end
  end

  def prepare_symlink_escape
    outside = Rails.root.join('storage/spec-hosted-restore-outside')
    FileUtils.mkdir_p(outside)
    File.symlink(outside, durable_root.join('escape'))
    outside
  end

  def unsafe_evidence_environments(outside)
    [
      temporary_evidence_environment,
      outside_evidence_environment(outside),
      symlink_evidence_environment,
      repository_tmp_environment
    ]
  end

  def temporary_evidence_environment
    environment.merge('EVIDENCE_ROOT' => Dir.tmpdir, 'EVIDENCE_OUTPUT' => File.join(Dir.tmpdir, 'evidence'))
  end

  def outside_evidence_environment(outside)
    environment.merge('EVIDENCE_OUTPUT' => outside.join('evidence').to_s)
  end

  def symlink_evidence_environment
    environment.merge('EVIDENCE_OUTPUT' => durable_root.join('escape/evidence').to_s)
  end

  def repository_tmp_environment
    repository_tmp = Rails.root.join('tmp')
    environment.merge(
      'EVIDENCE_ROOT' => repository_tmp.to_s,
      'EVIDENCE_OUTPUT' => repository_tmp.join('evidence').to_s
    )
  end

  def expect_invalid_locations(invalid_inputs)
    invalid_inputs.each do |invalid|
      expect { described_class.new(environment: invalid, runner: recording_runner(success_outputs)) }
        .to raise_error(HostedRestore::Input::Invalid)
    end
  end
end
