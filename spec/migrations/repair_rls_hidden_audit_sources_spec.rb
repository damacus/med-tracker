# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260709131000_create_tamper_evident_audit_ledger')
require Rails.root.join('db/migrate/20260709143000_configure_audit_object_lock_exporter')
require Rails.root.join('db/migrate/20260717120000_repair_rls_hidden_audit_sources')

RSpec.describe RepairRlsHiddenAuditSources do
  fixtures :accounts, :people, :users

  POLICY_NAME = 'repair_rls_hidden_audit_sources_select'

  let(:connection) { ActiveRecord::Base.connection }
  let(:household) { users(:admin).person.household }
  let(:migration) { described_class.new }

  it 'repairs forced-RLS omissions without changing existing audit evidence' do
    with_historical_omission do |event_id, version_id|
      expect(missing_entry?('security_audit_events', event_id)).to be(true)
      expect(missing_entry?('versions', version_id)).to be(true)

      live_entry = create_live_tail
      preserved_records = preserve_existing_audit_records

      as_med_tracker_owner { migration.migrate(:up) }

      repaired_entries = AuditLedgerEntry.where(source_table: 'security_audit_events', source_id: event_id)
        .or(AuditLedgerEntry.where(source_table: 'versions', source_id: version_id))
        .order(:sequence)
      expect(repaired_entries.pluck(:source_table, :source_id)).to eq([
        ['security_audit_events', event_id],
        ['versions', version_id]
      ])
      expect(repaired_entries.pluck(:epoch_kind).uniq).to eq(['legacy-repair'])
      expect(repaired_entries.pluck(:chain_epoch).uniq.one?).to be(true)
      expect_repair_checkpoint(repaired_entries.last)
      expect_pending_deliveries(repaired_entries)
      expect_live_tail_checkpoint(live_entry)
      expect_existing_audit_records_unchanged(preserved_records)
      expect_complete_source_coverage
      expect_clean_rls_state

      repaired_state = repair_state
      as_med_tracker_owner { migration.migrate(:up) }
      expect(repair_state).to eq(repaired_state)
      expect_clean_rls_state
    end
  end

  it 'rolls back its transient policies when the repair cannot finish' do
    with_historical_omission do
      live_entry = create_live_tail
      connection.execute(<<~SQL.squish)
        UPDATE audit_chain_heads SET last_hash = NULL WHERE chain_key = #{connection.quote(live_entry.chain_key)}
      SQL

      expect { migrate_in_new_transaction }.to raise_error(ActiveRecord::StatementInvalid)

      expect_clean_rls_state
    end
  end

  it 'runs through the shared login without DATABASE_ROLE' do
    with_historical_omission do |event_id, version_id|
      expect(connection.select_value('SELECT current_user')).to eq(connection.select_value('SELECT session_user'))

      without_database_role { migration.migrate(:up) }

      expect(missing_entry?('security_audit_events', event_id)).to be(false)
      expect(missing_entry?('versions', version_id)).to be(false)
      expect_complete_source_coverage
      expect_clean_rls_state
    end
  end

  private

  def with_historical_omission
    connection.transaction(requires_new: true) do
      event_id, version_id = rebuild_ledger_with_hidden_sources
      yield event_id, version_id
      raise ActiveRecord::Rollback
    end
  end

  def rebuild_ledger_with_hidden_sources
    ActiveRecord::Migration.suppress_messages do
      ConfigureAuditObjectLockExporter.new.down
      CreateTamperEvidentAuditLedger.new.down
      force_source_rls
      event_id, version_id = insert_hidden_sources
      clear_current_household
      as_med_tracker_owner { CreateTamperEvidentAuditLedger.new.up }
      remove_ledger_entry('versions', version_id)
      ConfigureAuditObjectLockExporter.new.up
      [event_id, version_id]
    end
  end

  def insert_hidden_sources
    set_current_household
    event = Audit::Event.record!(
      household:,
      event_type: 'audit.legacy.hidden',
      metadata: { outcome: 'success' }
    )
    [event.id, insert_version(event.created_at)]
  end

  def insert_version(occurred_at)
    connection.select_value(<<~SQL.squish)
      INSERT INTO versions (item_type, item_id, event, object, created_at, audit_context, household_id)
      VALUES (
        'LegacyAudit', 1, 'legacy.test', '{"outcome":"success"}',
        #{connection.quote(occurred_at)}, '{}', #{household.id}
      )
      RETURNING id
    SQL
  end

  def create_live_tail
    set_current_household
    event = Audit::Event.record!(
      household:,
      event_type: 'audit.live.before_repair',
      metadata: { outcome: 'success' }
    )
    clear_current_household
    AuditLedgerEntry.find_by!(source_table: 'security_audit_events', source_id: event.id)
  end

  def preserve_existing_audit_records
    [AuditLedgerEntry, AuditCheckpoint, AuditExportDelivery, AuditSigningKey].to_h do |model|
      [model, model.order(:id).to_h { |record| [record.id, record.attributes] }]
    end
  end

  def expect_existing_audit_records_unchanged(preserved_records)
    preserved_records.each do |model, records|
      records.each do |id, attributes|
        expect(model.find(id).attributes).to eq(attributes)
      end
    end
  end

  def expect_repair_checkpoint(entry)
    expect(AuditCheckpoint.find_by!(chain_epoch: entry.chain_epoch)).to have_attributes(
      checkpoint_kind: 'legacy-repair',
      sequence: entry.sequence,
      entry_hash: entry.entry_hash,
      signature: nil,
      audit_signing_key_id: nil
    )
  end

  def expect_pending_deliveries(entries)
    entries.each do |entry|
      expect(AuditExportDelivery.find_by!(audit_ledger_entry: entry)).to have_attributes(
        status: 'pending',
        attempts: 0,
        object_key: nil,
        checksum_sha256: nil,
        object_version_id: nil,
        delivered_at: nil
      )
    end
  end

  def expect_live_tail_checkpoint(entry)
    expect(AuditCheckpoint.find_by!(chain_epoch: entry.chain_epoch, sequence: entry.sequence)).to have_attributes(
      checkpoint_kind: 'pre-legacy-repair',
      entry_hash: entry.entry_hash
    )
  end

  def expect_complete_source_coverage
    expect(connection.select_value(<<~SQL.squish)).to eq(0)
      SELECT COUNT(*)
      FROM (
        SELECT sources.source_table, sources.source_id
        FROM (
          SELECT 'versions'::text source_table, id source_id FROM versions
          UNION ALL
          SELECT 'security_audit_events'::text, id FROM security_audit_events
        ) sources
        LEFT JOIN audit_ledger_entries entries
          ON entries.source_table = sources.source_table AND entries.source_id = sources.source_id
        GROUP BY sources.source_table, sources.source_id
        HAVING COUNT(entries.id) <> 1
      ) incomplete_sources
    SQL
  end

  def expect_clean_rls_state
    expect(forced_rls?('versions')).to be(true)
    expect(forced_rls?('security_audit_events')).to be(true)
    expect(connection.select_value(<<~SQL.squish)).to eq(0)
      SELECT COUNT(*) FROM pg_policies WHERE policyname = #{connection.quote(POLICY_NAME)}
    SQL
  end

  def repair_state
    {
      heads: AuditChainHead.order(:id).pluck(:id, :chain_epoch, :epoch_kind, :last_sequence, :last_hash),
      entries: AuditLedgerEntry.order(:id).pluck(:id, :chain_epoch, :epoch_kind, :sequence, :entry_hash),
      checkpoints: AuditCheckpoint.order(:id).pluck(:id, :chain_epoch, :checkpoint_kind, :sequence, :entry_hash),
      deliveries: AuditExportDelivery.order(:id).pluck(:id, :audit_ledger_entry_id, :status, :attempts)
    }
  end

  def missing_entry?(source_table, source_id)
    !AuditLedgerEntry.exists?(source_table:, source_id:)
  end

  def remove_ledger_entry(source_table, source_id)
    entry = AuditLedgerEntry.find_by!(source_table:, source_id:)
    connection.execute("DELETE FROM audit_export_deliveries WHERE audit_ledger_entry_id = #{entry.id}")
    connection.execute("DELETE FROM audit_ledger_entries WHERE id = #{entry.id}")
  end

  def force_source_rls
    connection.execute('ALTER TABLE versions FORCE ROW LEVEL SECURITY')
    connection.execute('ALTER TABLE security_audit_events FORCE ROW LEVEL SECURITY')
  end

  def forced_rls?(table_name)
    connection.select_value(<<~SQL.squish)
      SELECT relforcerowsecurity
      FROM pg_class
      WHERE oid = #{connection.quote(table_name)}::regclass
    SQL
  end

  def set_current_household
    connection.execute("SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)")
  end

  def clear_current_household
    connection.execute("SELECT set_config('med_tracker.current_household_id', '', true)")
  end

  def as_med_tracker_owner
    connection.execute('SET LOCAL ROLE med_tracker_owner')
    yield
  ensure
    connection.execute('RESET ROLE')
  end

  def migrate_in_new_transaction
    connection.transaction(requires_new: true) do
      connection.execute('SET LOCAL ROLE med_tracker_owner')
      migration.migrate(:up)
    end
  ensure
    connection.execute('RESET ROLE')
  end

  def without_database_role
    database_role = ENV.delete('DATABASE_ROLE')
    yield
  ensure
    ENV['DATABASE_ROLE'] = database_role if database_role
  end
end
