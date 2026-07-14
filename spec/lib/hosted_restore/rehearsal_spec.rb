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
  let(:output_root) { Rails.root.join('tmp/spec-hosted-restore-evidence') }
  let(:output) { output_root.join('rehearsal-2026-q3') }
  let(:environment) do
    {
      'DATABASE_BACKUP_ID' => 'database-snapshot-2026-07-14T010000Z',
      'ATTACHMENT_BACKUP_ID' => 'attachments-snapshot-2026-07-14T010000Z',
      'RESTORE_TARGET_ID' => 'isolated-restore-2026-q3',
      'APP_IMAGE' => 'ghcr.io/damacus/med-tracker:v0.5.0-rc1',
      'TESTER' => 'restore-operator',
      'HOUSEHOLD_A_ID' => '910001',
      'HOUSEHOLD_B_ID' => '920002',
      'WORM_REFERENCE' => 'object-lock-checkpoint-2026-07-14',
      'WORM_HEADS_JSON' => JSON.generate(
        sample_a: { chain_epoch: SecureRandom.uuid, sequence: 11, entry_hash: 'a' * 64 },
        sample_b: { chain_epoch: SecureRandom.uuid, sequence: 22, entry_hash: 'b' * 64 }
      ),
      'EVIDENCE_OUTPUT' => output.to_s
    }
  end
  let(:success_outputs) do
    {
      'owner.db_migrate' => { outcome: 'passed', schema_version: '20260714090000' },
      'runtime.restore_verify' => {
        outcome: 'passed', schema_version: '20260714090000', database_role: 'med_tracker_app',
        forced_rls: true, default_deny: true, isolation: { clinical: true, audit: true, attachments: true },
        storage: { samples_verified: 2 }
      },
      'audit.combined_verify' => {
        outcome: 'passed', scope: 'combined', samples_verified: 2, checked_entries: 8,
        checked_checkpoints: 2, checked_objects: 10, worm_comparison: 'match'
      }
    }
  end

  before do
    FileUtils.rm_rf(output_root)
    FileUtils.mkdir_p(output_root)
  end

  after { FileUtils.rm_rf(output_root) }

  it 'runs owner migration before forced-RLS and combined audit verification' do
    runner = recording_runner(success_outputs)

    result = described_class.new(environment:, runner:, clock: -> { Time.utc(2026, 7, 14, 9) }).call

    expect(result).to eq(0)
    expect(runner.command_ids).to eq(%w[owner.db_migrate runtime.restore_verify audit.combined_verify])
    expect(JSON.parse(output.join('evidence.json').read).fetch('outcome')).to eq('passed')
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

  it 'refuses missing, placeholder, ambiguous, or overwrite-prone inputs before running commands' do
    invalid_inputs = [
      environment.except('DATABASE_BACKUP_ID'),
      environment.merge('APP_IMAGE' => 'ghcr.io/damacus/med-tracker:latest'),
      environment.merge('HOUSEHOLD_B_ID' => environment.fetch('HOUSEHOLD_A_ID'))
    ]

    invalid_inputs.each do |invalid|
      runner = recording_runner(success_outputs)
      expect { described_class.new(environment: invalid, runner:).call }
        .to raise_error(HostedRestore::Input::Invalid)
      expect(runner.command_ids).to be_empty
    end

    FileUtils.mkdir_p(output)
    expect do
      described_class.new(environment:, runner: recording_runner(success_outputs)).call
    end.to raise_error(HostedRestore::EvidenceWriter::AlreadyExists)
  end

  def recording_runner(outputs, failure: nil)
    HostedRestoreSpec::RecordingRunner.new(outputs, failure)
  end
end
