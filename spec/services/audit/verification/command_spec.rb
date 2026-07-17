# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Verification::Command do
  let(:output) { StringIO.new }
  let(:error_output) { StringIO.new }

  it 'emits stable JSON and returns zero for valid evidence' do
    result = Audit::Verification::Result.new(scope: 'database', checked_entries: 2, checked_checkpoints: 1,
                                             checked_objects: 0, issues: [])
    verifier = instance_double(Audit::Verification::DatabaseVerifier, call: result)

    exit_code = described_class.new(environment: { 'FORMAT' => 'json', 'SCOPE' => 'database' },
                                    output:, error_output:, database_verifier: verifier).call

    expect(exit_code).to eq(0)
    expect(JSON.parse(output.string)).to eq(
      'scope' => 'database', 'valid' => true, 'exit_code' => 0,
      'checked_entries' => 2, 'checked_checkpoints' => 1, 'checked_objects' => 0, 'issues' => []
    )
  end

  it 'returns one for integrity failures' do
    issue = Audit::Verification::Issue.new(code: 'entry_hash_mismatch', message: 'entry hash differs',
                                           chain_key: 'global', sequence: 1)
    result = Audit::Verification::Result.new(scope: 'database', checked_entries: 1, checked_checkpoints: 0,
                                             checked_objects: 0, issues: [issue])
    verifier = instance_double(Audit::Verification::DatabaseVerifier, call: result)

    exit_code = described_class.new(environment: { 'FORMAT' => 'json' }, output:, error_output:,
                                    database_verifier: verifier).call

    expect(exit_code).to eq(1)
    expect(JSON.parse(output.string).fetch('issues').first.fetch('code')).to eq('entry_hash_mismatch')
  end

  it 'returns two with PHI-safe output for invalid configuration or runtime failure' do
    verifier = instance_double(Audit::Verification::DatabaseVerifier)
    allow(verifier).to receive(:call).and_raise(Audit::Verification::ConfigurationError, 'invalid FROM')

    exit_code = described_class.new(environment: {}, output:, error_output:, database_verifier: verifier).call

    expect(exit_code).to eq(2)
    expect(error_output.string).to include('audit verification could not run: invalid FROM')
    expect(output.string).not_to include('VALID')
  end

  it 'rejects time filters for database and combined verification without reporting valid evidence' do
    %w[database combined].each do |scope|
      output.truncate(0)
      error_output.truncate(0)

      exit_code = described_class.new(
        environment: { 'SCOPE' => scope, 'FROM' => '2026-07-01T00:00:00Z' },
        output:, error_output:
      ).call

      expect(exit_code).to eq(2)
      expect(output.string).not_to include('VALID')
      expect(error_output.string).to include('time filters are unsupported for database verification')
    end
  end

  it 'preserves time filtering for WORM-only verification' do
    result = Audit::Verification::Result.new(scope: 'worm', checked_entries: 0, checked_checkpoints: 0,
                                             checked_objects: 1, issues: [])
    verifier = instance_double(Audit::Verification::WormVerifier, call: result)

    exit_code = described_class.new(
      environment: { 'SCOPE' => 'worm', 'FROM' => '2026-07-01T00:00:00Z' },
      output:, error_output:, worm_verifier: verifier
    ).call

    expect(exit_code).to eq(0)
    expect(output.string).to include('VALID')
  end
end
