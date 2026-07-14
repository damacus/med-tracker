# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HostedRestore::AuditVerifier do
  let(:heads) do
    {
      'sample_a' => { 'chain_epoch' => SecureRandom.uuid, 'sequence' => 11, 'entry_hash' => 'a' * 64 },
      'sample_b' => { 'chain_epoch' => SecureRandom.uuid, 'sequence' => 22, 'entry_hash' => 'b' * 64 }
    }
  end
  let(:command) do
    instance_double(
      HostedRestore::CombinedAuditCommand,
      call: { checked_entries: 4, checked_checkpoints: 1, checked_objects: 5, issue_codes: [] }
    )
  end
  let(:head_comparator) { instance_double(HostedRestore::WormHeadComparator, call: true) }

  it 'combines database, signed checkpoint, Object Lock, and external WORM-head comparison for both samples' do
    result = described_class.new(
      household_ids: [101, 202], expected_heads: heads, command:, head_comparator:,
      current_role: -> { 'med_tracker_audit_verifier' }
    ).call

    expect(result).to include(
      scope: 'combined', samples_verified: 2, checked_entries: 8,
      checked_checkpoints: 2, checked_objects: 10, worm_comparison: 'match'
    )
    expect(head_comparator).to have_received(:call).twice
  end

  it 'fails on combined verification issues or restored/WORM head divergence' do
    allow(command).to receive(:call).and_return(
      checked_entries: 1, checked_checkpoints: 0, checked_objects: 0, issue_codes: ['worm_object_invalid']
    )
    expect do
      described_class.new(household_ids: [101, 202], expected_heads: heads, command:, head_comparator:,
                          current_role: -> { 'med_tracker_audit_verifier' }).call
    end.to raise_error(HostedRestore::VerificationError, 'audit_worm_verification_failed')

    allow(command).to receive(:call).and_return(
      checked_entries: 4, checked_checkpoints: 1, checked_objects: 5, issue_codes: []
    )
    allow(head_comparator).to receive(:call).and_return(false)
    expect do
      described_class.new(household_ids: [101, 202], expected_heads: heads, command:, head_comparator:,
                          current_role: -> { 'med_tracker_audit_verifier' }).call
    end.to raise_error(HostedRestore::VerificationError, 'worm_restore_divergence')
  end

  it 'requires two distinct restored household samples when invoked directly' do
    expect do
      described_class.new(household_ids: [101, 101], expected_heads: heads, command:, head_comparator:,
                          current_role: -> { 'med_tracker_audit_verifier' }).call
    end.to raise_error(HostedRestore::VerificationError, 'distinct_household_samples_required')
  end
end
