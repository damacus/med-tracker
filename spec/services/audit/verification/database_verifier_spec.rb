# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Verification::DatabaseVerifier do
  fixtures :accounts, :people, :users

  let(:household) { users(:admin).person.household }

  it 'validates source rows, canonical payloads, hash links, chain heads, and signed checkpoints' do
    first = ledger_entry_for('audit.verify.first')
    second = ledger_entry_for('audit.verify.second')
    signer.sign(second)

    result = described_class.new(entries: AuditLedgerEntry.where(id: [first.id, second.id])).call

    expect(result).to be_valid
    expect(result.checked_entries).to eq(2)
    expect(result.checked_checkpoints).to eq(1)
  end

  it 'reports a modified entry hash' do
    entry = ledger_entry_for('audit.verify.hash')
    execute("UPDATE audit_ledger_entries SET entry_hash = decode('#{'00' * 32}', 'hex') WHERE id = #{entry.id}")

    result = described_class.new(entries: AuditLedgerEntry.where(id: entry.id)).call

    expect(result.issue_codes).to include('entry_hash_mismatch', 'chain_head_mismatch')
    expect(result.exit_code).to eq(1)
  end

  it 'reports source-row divergence' do
    entry = ledger_entry_for('audit.verify.source')
    execute("UPDATE security_audit_events SET event_type = 'audit.verify.changed' WHERE id = #{entry.source_id}")

    result = described_class.new(entries: AuditLedgerEntry.where(id: entry.id)).call

    expect(result.issue_codes).to include('source_payload_mismatch')
  end

  it 'reports missing or duplicated sequence positions' do
    first = ledger_entry_for('audit.verify.sequence.first')
    second = ledger_entry_for('audit.verify.sequence.second')
    missing = second.dup
    missing.sequence += 2
    duplicate = second.dup
    duplicate.sequence = first.sequence

    missing_result = described_class.new(entries: [missing], verify_heads: false).call
    duplicated = described_class.new(entries: [first, duplicate], verify_heads: false).call

    expect(missing_result.issue_codes).to include('sequence_gap')
    expect(duplicated.issue_codes).to include('sequence_gap')
  end

  it 'validates independent household chains together' do
    first = ledger_entry_for('audit.verify.mixed.first')
    other_household = Household.create!(name: 'Independent verifier household')
    event = Audit::Event.record!(household: other_household, event_type: 'audit.verify.mixed.second',
                                 metadata: { outcome: 'success' })
    second = AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)

    result = described_class.new(entries: [second, first]).call

    expect(result).to be_valid
  end

  it 'reports a deleted chain tail instead of accepting the shortened ledger' do
    entry = ledger_entry_for('audit.verify.truncated')
    execute("DELETE FROM audit_export_deliveries WHERE audit_ledger_entry_id = #{entry.id}")
    execute("DELETE FROM audit_ledger_entries WHERE id = #{entry.id}")

    result = described_class.new(entries: AuditLedgerEntry.none).call

    expect(result.issue_codes).to include('chain_head_mismatch')
  end

  it 'reports unsigned and invalid checkpoint evidence' do
    entry = ledger_entry_for('audit.verify.checkpoint')
    checkpoint = AuditCheckpoint.create!(
      household:, chain_key: entry.chain_key, chain_epoch: entry.chain_epoch,
      checkpoint_kind: 'periodic', sequence: entry.sequence, entry_hash: entry.entry_hash
    )

    unsigned = described_class.new(entries: AuditLedgerEntry.where(id: entry.id)).call
    expect(unsigned.issue_codes).to include('checkpoint_unsigned')

    signer.sign(entry)
    execute("UPDATE audit_checkpoints SET signature = decode('00', 'hex') WHERE id = #{checkpoint.id}")
    invalid = described_class.new(entries: AuditLedgerEntry.where(id: entry.id)).call
    expect(invalid.issue_codes).to include('checkpoint_signature_invalid')
  end

  private

  def signer
    @signer ||= Audit::CheckpointSigner.new(
      key_id: 'verification-key', private_key_pem: OpenSSL::PKey.generate_key('ED25519').private_to_pem
    )
  end

  def ledger_entry_for(event_type)
    event = Audit::Event.record!(household:, event_type:, metadata: { outcome: 'success' })
    AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
  end

  def execute(sql)
    ActiveRecord::Base.connection.execute(sql)
  end
end
