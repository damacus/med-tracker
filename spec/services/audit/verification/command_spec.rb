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
  end
end
