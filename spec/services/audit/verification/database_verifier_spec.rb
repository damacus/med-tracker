# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Verification::DatabaseVerifier do
  fixtures :accounts, :people, :users

  let(:household) { users(:admin).person.household }

  it 'validates source rows, canonical payloads, hash links, chain heads, and signed checkpoints' do
    first = ledger_entry_for('audit.verify.first')
    second = ledger_entry_for('audit.verify.second')
    signer.sign(second)

    result = verify(entries: AuditLedgerEntry.where(id: [first.id, second.id]))

    expect(result).to be_valid
    expect(result.checked_entries).to eq(2)
    expect(result.checked_checkpoints).to eq(1)
  end

  it 'reports a modified entry hash' do
    entry = ledger_entry_for('audit.verify.hash')
    execute("UPDATE audit_ledger_entries SET entry_hash = decode('#{'00' * 32}', 'hex') WHERE id = #{entry.id}")

    result = verify(entries: AuditLedgerEntry.where(id: entry.id))

    expect(result.issue_codes).to include('entry_hash_mismatch', 'chain_head_mismatch')
    expect(result.exit_code).to eq(1)
  end

  it 'reports source-row divergence' do
    entry = ledger_entry_for('audit.verify.source')
    execute("UPDATE security_audit_events SET event_type = 'audit.verify.changed' WHERE id = #{entry.source_id}")

    result = verify(entries: AuditLedgerEntry.where(id: entry.id))

    expect(result.issue_codes).to include('source_payload_mismatch')
  end

  it 'reports a source row with no matching ledger entry without exposing its payload' do
    event = Audit::Event.record!(household:, event_type: 'audit.verify.missing', metadata: { outcome: 'success' })
    entry = AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
    execute("DELETE FROM audit_export_deliveries WHERE audit_ledger_entry_id = #{entry.id}")
    execute("DELETE FROM audit_ledger_entries WHERE id = #{entry.id}")

    result = verify(entries: AuditLedgerEntry.where(household:))

    issue = result.issues.find { |candidate| candidate.code == 'source_ledger_entry_missing' }
    expect(issue&.to_h).to include(
      code: 'source_ledger_entry_missing',
      metadata: { source_table: 'security_audit_events', missing_count: 1 }
    )
    expect(issue.to_h.to_s).not_to include(event.event_type)
  end

  it 'reports a version row with no matching ledger entry' do
    version_id = insert_version('audit.verify.version.missing', household.id)
    remove_ledger_entry('versions', version_id)

    result = verify(entries: AuditLedgerEntry.where(household:))

    expect(result.issues.map(&:to_h)).to include(
      hash_including(
        code: 'source_ledger_entry_missing',
        metadata: { source_table: 'versions', missing_count: 1 }
      )
    )
  end

  it 'limits source completeness to the requested household' do
    own_event = Audit::Event.record!(household:, event_type: 'audit.verify.household.missing', metadata: {})
    other_household = Household.create!(name: 'Unrelated verifier household')
    other_event = Audit::Event.record!(household: other_household, event_type: 'audit.verify.unrelated', metadata: {})
    remove_ledger_entry('security_audit_events', own_event.id)
    remove_ledger_entry('security_audit_events', other_event.id)

    result = verify(entries: AuditLedgerEntry.where(household:), household_id: household.id)

    expect(result.issues.map(&:to_h)).to include(
      hash_including(
        code: 'source_ledger_entry_missing',
        metadata: { source_table: 'security_audit_events', missing_count: 1 }
      )
    )
  end

  it 'scopes every verification phase when the requested household has no ledger entries' do
    requested_household = Household.create!(name: 'Empty requested verifier household')
    unrelated_entry, unrelated_event = corrupt_unrelated_audit_evidence

    result = verify(entries: AuditLedgerEntry.all, household_id: requested_household.id)

    expect(result).to be_valid
    expect(result.checked_entries).to eq(0)
    expect(result.checked_checkpoints).to eq(0)
    expect(result.to_h.to_s).not_to include(unrelated_entry.chain_key, unrelated_event.event_type)
  end

  it 'binds household completeness filters as bigint values' do
    large_household_id = 4_294_967_296
    large_household = Household.create!(id: large_household_id, name: 'Bigint verifier household')
    event = Audit::Event.record!(household: large_household, event_type: 'audit.verify.bigint', metadata: {})
    remove_ledger_entry('security_audit_events', event.id)

    result = verify(entries: AuditLedgerEntry.where(household: large_household), household_id: large_household_id)

    expect(result.issues.map(&:to_h)).to include(
      hash_including(metadata: { source_table: 'security_audit_events', missing_count: 1 })
    )
  end

  it 'requires the dedicated verifier database role' do
    expect do
      described_class.new(entries: AuditLedgerEntry.none).call
    end.to raise_error(Audit::Verification::ConfigurationError, /med_tracker_audit_verifier/)
  end

  it 'requires every read privilege used by verification' do
    execute('REVOKE SELECT ON versions FROM med_tracker_audit_verifier')

    expect do
      verify(entries: AuditLedgerEntry.none)
    end.to raise_error(Audit::Verification::ConfigurationError, /SELECT privilege/)
  ensure
    execute('GRANT SELECT ON versions TO med_tracker_audit_verifier')
  end

  it 'requires the complete verifier RLS policy' do
    execute('DROP POLICY audit_verifier_complete_visibility ON security_audit_events')

    expect do
      verify(entries: AuditLedgerEntry.none)
    end.to raise_error(Audit::Verification::ConfigurationError, /RLS policy/)
  ensure
    install_verifier_visibility_policy
  end

  it 'rejects effective audit mutation privileges' do
    execute('GRANT INSERT ON audit_ledger_entries TO med_tracker_audit_verifier')

    expect do
      verify(entries: AuditLedgerEntry.none)
    end.to raise_error(Audit::Verification::ConfigurationError, /mutation privilege/)
  ensure
    execute('REVOKE INSERT ON audit_ledger_entries FROM med_tracker_audit_verifier')
  end

  it 'rejects effective reads on unapproved public tables' do
    execute('GRANT SELECT ON medications TO med_tracker_audit_verifier')

    expect do
      verify(entries: AuditLedgerEntry.none)
    end.to raise_error(Audit::Verification::ConfigurationError, /unapproved SELECT privilege/)
  ensure
    execute('REVOKE SELECT ON medications FROM med_tracker_audit_verifier')
  end

  it 'rejects membership in runtime, owner, or exporter roles' do
    %w[med_tracker_app med_tracker_owner med_tracker_audit_exporter].each do |role_name|
      execute("GRANT #{role_name} TO med_tracker_audit_verifier")

      expect do
        verify(entries: AuditLedgerEntry.none)
      end.to raise_error(Audit::Verification::ConfigurationError, /forbidden role membership/)
    ensure
      execute("REVOKE #{role_name} FROM med_tracker_audit_verifier")
    end
  end

  it 'rejects competing unrestricted security-event policies for other roles' do
    execute <<~SQL.squish
      CREATE POLICY audit_verifier_competing_visibility ON security_audit_events
      FOR SELECT TO med_tracker_app
      USING (true)
    SQL

    expect do
      verify(entries: AuditLedgerEntry.none)
    end.to raise_error(Audit::Verification::ConfigurationError, /competing unrestricted RLS policy/)
  ensure
    execute('DROP POLICY IF EXISTS audit_verifier_competing_visibility ON security_audit_events')
  end

  it 'reports missing or duplicated sequence positions' do
    first = ledger_entry_for('audit.verify.sequence.first')
    second = ledger_entry_for('audit.verify.sequence.second')
    missing = second.dup
    missing.sequence += 2
    duplicate = second.dup
    duplicate.sequence = first.sequence

    missing_result = verify(entries: [missing], verify_heads: false)
    duplicated = verify(entries: [first, duplicate], verify_heads: false)

    expect(missing_result.issue_codes).to include('sequence_gap')
    expect(duplicated.issue_codes).to include('sequence_gap')
  end

  it 'validates independent household chains together' do
    first = ledger_entry_for('audit.verify.mixed.first')
    other_household = Household.create!(name: 'Independent verifier household')
    event = Audit::Event.record!(household: other_household, event_type: 'audit.verify.mixed.second',
                                 metadata: { outcome: 'success' })
    second = AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)

    result = verify(entries: [second, first])

    expect(result).to be_valid
  end

  it 'reports a deleted chain tail instead of accepting the shortened ledger' do
    entry = ledger_entry_for('audit.verify.truncated')
    execute("DELETE FROM audit_export_deliveries WHERE audit_ledger_entry_id = #{entry.id}")
    execute("DELETE FROM audit_ledger_entries WHERE id = #{entry.id}")

    result = verify(entries: AuditLedgerEntry.none)

    expect(result.issue_codes).to include('chain_head_mismatch')
  end

  it 'reports unsigned and invalid checkpoint evidence' do
    entry = ledger_entry_for('audit.verify.checkpoint')
    checkpoint = AuditCheckpoint.create!(
      household:, chain_key: entry.chain_key, chain_epoch: entry.chain_epoch,
      checkpoint_kind: 'periodic', sequence: entry.sequence, entry_hash: entry.entry_hash
    )

    unsigned = verify(entries: AuditLedgerEntry.where(id: entry.id))
    expect(unsigned.issue_codes).to include('checkpoint_unsigned')

    signer.sign(entry)
    execute("UPDATE audit_checkpoints SET signature = decode('00', 'hex') WHERE id = #{checkpoint.id}")
    invalid = verify(entries: AuditLedgerEntry.where(id: entry.id))
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

  def verify(entries:, verify_heads: true, household_id: nil)
    with_audit_verifier_role do
      described_class.new(entries:, verify_heads:, household_id:).call
    end
  end

  def remove_ledger_entry(source_table, source_id)
    entry = AuditLedgerEntry.find_by!(source_table:, source_id:)
    execute("DELETE FROM audit_export_deliveries WHERE audit_ledger_entry_id = #{entry.id}")
    execute("DELETE FROM audit_ledger_entries WHERE id = #{entry.id}")
  end

  def insert_version(event, household_id)
    ActiveRecord::Base.connection.select_value(<<~SQL.squish)
      INSERT INTO versions (item_type, item_id, event, object, created_at, audit_context, household_id)
      VALUES ('VerifierSpec', 1, #{ActiveRecord::Base.connection.quote(event)}, '{}', clock_timestamp(), '{}',
              #{household_id})
      RETURNING id
    SQL
  end

  def corrupt_unrelated_audit_evidence
    unrelated_household = Household.create!(name: 'Corrupted unrelated verifier household')
    event = Audit::Event.record!(
      household: unrelated_household, event_type: 'audit.verify.unrelated.secret', metadata: {}
    )
    entry = AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
    AuditCheckpoint.create!(
      household: unrelated_household, chain_key: entry.chain_key, chain_epoch: entry.chain_epoch,
      checkpoint_kind: 'periodic', sequence: entry.sequence, entry_hash: entry.entry_hash
    )
    execute(<<~SQL.squish)
      UPDATE audit_chain_heads
      SET last_hash = decode('#{'00' * 32}', 'hex')
      WHERE household_id = #{unrelated_household.id}
    SQL
    [entry, event]
  end

  def install_verifier_visibility_policy
    execute <<~SQL.squish
      CREATE POLICY audit_verifier_complete_visibility ON security_audit_events
      FOR SELECT TO med_tracker_audit_verifier
      USING (true)
    SQL
  end

  def with_audit_verifier_role
    execute("SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)")
    execute('SET LOCAL ROLE med_tracker_audit_verifier')
    yield
  ensure
    execute('RESET ROLE')
    execute("SELECT set_config('med_tracker.current_household_id', '', true)")
  end
end
