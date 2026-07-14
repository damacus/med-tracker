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
      call: {
        checked_entries: 4, checked_checkpoints: 1, checked_objects: 5, issue_codes: [], verified_head: true
      }
    )
  end

  it 'combines database, signed checkpoint, Object Lock, and external WORM-head comparison for both samples' do
    result = described_class.new(
      household_ids: [101, 202], expected_heads: heads, command:,
      current_role: -> { 'med_tracker_audit_verifier' }
    ).call

    expect(result).to include(
      scope: 'combined', samples_verified: 2, checked_entries: 8,
      checked_checkpoints: 2, checked_objects: 10, verified_heads: 2, worm_comparison: 'match'
    )
    expect(command).to have_received(:call).twice
  end

  it 'fails on combined verification issues or restored/WORM head divergence' do
    allow(command).to receive(:call).and_return(
      checked_entries: 1, checked_checkpoints: 0, checked_objects: 0,
      issue_codes: ['worm_object_invalid'], verified_head: false
    )
    expect do
      described_class.new(household_ids: [101, 202], expected_heads: heads, command:,
                          current_role: -> { 'med_tracker_audit_verifier' }).call
    end.to raise_error(HostedRestore::VerificationError, 'audit_worm_verification_failed')

    allow(command).to receive(:call).and_return(
      checked_entries: 4, checked_checkpoints: 1, checked_objects: 5, issue_codes: [], verified_head: false
    )
    expect do
      described_class.new(household_ids: [101, 202], expected_heads: heads, command:,
                          current_role: -> { 'med_tracker_audit_verifier' }).call
    end.to raise_error(HostedRestore::VerificationError, 'worm_restore_divergence')
  end

  it 'requires two distinct restored household samples when invoked directly' do
    expect do
      described_class.new(household_ids: [101, 101], expected_heads: heads, command:,
                          current_role: -> { 'med_tracker_audit_verifier' }).call
    end.to raise_error(HostedRestore::VerificationError, 'distinct_household_samples_required')
  end

  context 'with the real audit verifier role and combined database command' do
    let(:households) { create_list(:household, 2) }

    it 'sets tenant context per sample and verifies the exact restored ledger/checkpoint heads' do
      expected_heads = households.to_h do |household|
        entry = ledger_entry_for(household)
        checkpoint = signer.sign(entry)
        mark_delivered(delivery_for(entry), checkpoint.audit_export_delivery)
        [household.id, expected_head(checkpoint)]
      end

      result = with_audit_verifier_role do
        described_class.new(
          household_ids: households.map(&:id), expected_heads: sample_heads(expected_heads),
          command: combined_command
        ).call
      end

      expect(result).to include(worm_comparison: 'match', verified_heads: 2)
    end

    it 'fails when the external head has no exact restored ledger and checkpoint evidence' do
      expected_heads = households.to_h do |household|
        entry = ledger_entry_for(household)
        checkpoint = signer.sign(entry)
        mark_delivered(delivery_for(entry), checkpoint.audit_export_delivery)
        [household.id, expected_head(checkpoint)]
      end
      expected_heads.fetch(households.first.id)['sequence'] += 1

      expect do
        with_audit_verifier_role do
          described_class.new(
            household_ids: households.map(&:id), expected_heads: sample_heads(expected_heads),
            command: combined_command
          ).call
        end
      end.to raise_error(HostedRestore::VerificationError, 'expected_head_evidence_missing')
    end

    def ledger_entry_for(household)
      event = Audit::Event.record!(
        household:, event_type: 'hosted_restore.integration', metadata: { outcome: 'success' }
      )
      AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
    end

    def signer
      @signer ||= Audit::CheckpointSigner.new(
        key_id: "restore-integration-#{SecureRandom.hex(4)}",
        private_key_pem: OpenSSL::PKey.generate_key('ED25519').private_to_pem
      )
    end

    def combined_command
      @combined_command ||= HostedRestore::CombinedAuditCommand.new(
        environment: {}, worm_verifier_factory: ->(_environment) { worm_verifier }
      )
    end

    def worm_verifier
      result = Audit::Verification::Result.new(
        scope: 'worm', checked_entries: 0, checked_checkpoints: 0, checked_objects: 2, issues: []
      )
      instance_double(Audit::Verification::WormVerifier, call: result)
    end

    def mark_delivered(*deliveries)
      deliveries.each { |delivery| delivery.update!(status: 'delivered') }
    end

    def delivery_for(entry)
      AuditExportDelivery.find_by!(audit_ledger_entry: entry)
    end

    def expected_head(checkpoint)
      {
        'chain_epoch' => checkpoint.chain_epoch,
        'sequence' => checkpoint.sequence,
        'entry_hash' => checkpoint.entry_hash.unpack1('H*')
      }
    end

    def sample_heads(heads_by_household)
      {
        'sample_a' => heads_by_household.fetch(households.first.id),
        'sample_b' => heads_by_household.fetch(households.second.id)
      }
    end

    def with_audit_verifier_role(&)
      ActiveRecord::Base.connection.transaction(requires_new: true) do
        ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_audit_verifier')
        yield
      end
    end
  end
end
