# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::CheckpointSigner do
  fixtures :accounts, :people, :users

  let(:private_key) { OpenSSL::PKey.generate_key('ED25519') }
  let(:signer) { described_class.new(key_id: 'audit-key-2026-01', private_key_pem: private_key.private_to_pem) }

  it 'records a verifiable signed checkpoint without storing the private key' do
    event = Audit::Event.record!(household: users(:admin).person.household, event_type: 'audit.checkpoint.test')
    entry = AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)

    checkpoint = signer.sign(entry)

    expect(checkpoint).to have_attributes(sequence: entry.sequence, entry_hash: entry.entry_hash,
                                          signed_at: be_present)
    expect(checkpoint.audit_signing_key).to have_attributes(key_id: 'audit-key-2026-01', algorithm: 'ed25519')
    expect(checkpoint.audit_signing_key.attributes.keys).not_to include('private_key')
    public_key = OpenSSL::PKey.read(private_key.public_to_pem)
    expect(public_key.verify(nil, checkpoint.signature, signer.payload_for(entry))).to be(true)
  end

  it 'does not replace an existing signature with another key' do
    event = Audit::Event.record!(household: users(:admin).person.household, event_type: 'audit.checkpoint.once')
    entry = AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
    checkpoint = signer.sign(entry)
    replacement = described_class.new(key_id: 'replacement',
                                      private_key_pem: OpenSSL::PKey.generate_key('ED25519').private_to_pem)

    expect { replacement.sign(entry) }.to raise_error(described_class::AlreadySigned)
    expect(checkpoint.reload.audit_signing_key.key_id).to eq('audit-key-2026-01')
  end

  it 'retains old public keys after rotating to a new signing key' do
    first_entry = ledger_entry_for('audit.checkpoint.first-key')
    second_entry = ledger_entry_for('audit.checkpoint.second-key')
    signer.sign(first_entry)
    replacement = described_class.new(key_id: 'audit-key-2026-02',
                                      private_key_pem: OpenSSL::PKey.generate_key('ED25519').private_to_pem)

    replacement.sign(second_entry)

    expect(AuditSigningKey.where(key_id: %w[audit-key-2026-01 audit-key-2026-02]).count).to eq(2)
  end

  it 'preserves the legacy-baseline label when signing a migration checkpoint' do
    entry = ledger_entry_for('audit.checkpoint.legacy-label')
    AuditCheckpoint.create!(
      household: entry.household, chain_key: entry.chain_key, chain_epoch: entry.chain_epoch,
      checkpoint_kind: 'legacy-baseline', sequence: entry.sequence, entry_hash: entry.entry_hash
    )

    checkpoint = signer.sign(entry)

    expect(checkpoint.checkpoint_kind).to eq('legacy-baseline')
  end

  def ledger_entry_for(event_type)
    event = Audit::Event.record!(household: users(:admin).person.household, event_type:)
    AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
  end
end
