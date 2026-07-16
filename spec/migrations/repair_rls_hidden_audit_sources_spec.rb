# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260709131000_create_tamper_evident_audit_ledger')
require Rails.root.join('db/migrate/20260709143000_configure_audit_object_lock_exporter')
require Rails.root.join('db/migrate/20260717120000_repair_rls_hidden_audit_sources')

RSpec.describe RepairRlsHiddenAuditSources do
  fixtures :accounts, :people, :users

  POLICY_NAME = 'repair_rls_hidden_audit_sources_select'
  SHARED_LOGIN = 'audit_repair_shared_login'

  let(:connection) { ActiveRecord::Base.connection }
  let(:household) { users(:admin).person.household }
  let(:migration) { described_class.new }

  it 'repairs forced-RLS omissions without changing existing audit evidence' do
    with_historical_omission do |event_id, version_id|
      expect(missing_entry?('security_audit_events', event_id)).to be(true)
      expect(missing_entry?('versions', version_id)).to be(true)

      expected_order = add_ordering_sources(event_id, version_id)
      live_entry = create_live_tail
      preserved_records = preserve_existing_audit_records

      as_med_tracker_owner { migration.migrate(:up) }

      repaired_entries = entries_for(expected_order).order(:id)
      expect(repaired_entries.pluck(:chain_key, :source_table, :source_id)).to eq(expected_order)
      expect(repaired_entries.pluck(:epoch_kind).uniq).to eq(['legacy-repair'])
      expect_repair_checkpoints(repaired_entries)
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
      create_live_tail
      install_repair_checkpoint_failure
      state_before_repair = repair_state

      expect { migrate_in_new_transaction }.to raise_error(
        ActiveRecord::StatementInvalid, /injected failure after repaired entry and delivery/
      )

      expect(repair_state).to eq(state_before_repair)
      expect_clean_rls_state
    end
  end

  it 'rejects a conflicting existing live-tail checkpoint before rotation' do
    with_historical_omission do
      live_entry = create_live_tail
      create_live_tail_checkpoint(live_entry, entry_hash: "\0".b * 32)
      state_before_repair = repair_state

      expect { migrate_in_new_transaction }.to raise_error(
        ActiveRecord::StatementInvalid, /existing checkpoint does not match the pre-repair live tail/
      )

      expect(repair_state).to eq(state_before_repair)
      expect_clean_rls_state
    end
  end

  it 'preserves a matching existing live-tail checkpoint' do
    with_historical_omission do
      live_entry = create_live_tail
      checkpoint = create_live_tail_checkpoint(live_entry, entry_hash: live_entry.entry_hash)
      attributes = checkpoint.attributes

      as_med_tracker_owner { migration.migrate(:up) }

      expect(checkpoint.reload.attributes).to eq(attributes)
      expect_complete_source_coverage
      expect_clean_rls_state
    end
  end

  it 'runs through the shared login without DATABASE_ROLE' do
    with_historical_omission do |event_id, version_id|
      as_restricted_shared_login do
        expect_restricted_shared_login
        without_database_role { migration.migrate(:up) }
        expect_restricted_shared_login
      end

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
    insert_version_for(occurred_at, household.id)
  end

  def insert_version_for(occurred_at, household_id)
    connection.select_value(<<~SQL.squish)
      INSERT INTO versions (item_type, item_id, event, object, created_at, audit_context, household_id)
      VALUES (
        'LegacyAudit', 1, 'legacy.test', '{"outcome":"success"}',
        #{connection.quote(occurred_at)}, '{}', #{connection.quote(household_id)}
      )
      RETURNING id
    SQL
  end

  def add_ordering_sources(event_id, version_id)
    occurred_at = connection.select_value(<<~SQL.squish)
      SELECT created_at FROM security_audit_events WHERE id = #{event_id}
    SQL
    connection.execute('ALTER TABLE versions DISABLE TRIGGER append_versions_to_audit_ledger')
    second_version_id = insert_version_for(occurred_at, household.id)
    global_version_id = insert_version_for(occurred_at, nil)
    connection.execute('ALTER TABLE versions ENABLE TRIGGER append_versions_to_audit_ledger')

    [
      ['global', 'versions', global_version_id],
      ["household:#{household.id}", 'security_audit_events', event_id],
      ["household:#{household.id}", 'versions', version_id],
      ["household:#{household.id}", 'versions', second_version_id]
    ]
  ensure
    connection.execute('ALTER TABLE versions ENABLE TRIGGER append_versions_to_audit_ledger')
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

  def expect_repair_checkpoints(entries)
    entries.group_by(&:chain_epoch).each_value do |epoch_entries|
      entry = epoch_entries.max_by(&:sequence)
      expect(AuditCheckpoint.find_by!(chain_epoch: entry.chain_epoch)).to have_attributes(
        checkpoint_kind: 'legacy-repair',
        sequence: entry.sequence,
        entry_hash: entry.entry_hash,
        signature: nil,
        audit_signing_key_id: nil
      )
    end
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
      heads: AuditChainHead.order(:id).map(&:attributes),
      entries: AuditLedgerEntry.order(:id).map(&:attributes),
      checkpoints: AuditCheckpoint.order(:id).map(&:attributes),
      deliveries: AuditExportDelivery.order(:id).map(&:attributes),
      policies: connection.select_rows(<<~SQL.squish)
        SELECT tablename, policyname, roles, cmd, qual
        FROM pg_policies
        WHERE tablename IN ('versions', 'security_audit_events')
        ORDER BY tablename, policyname
      SQL
    }
  end

  def entries_for(expected_order)
    expected_order.reduce(AuditLedgerEntry.none) do |entries, (_chain_key, source_table, source_id)|
      entries.or(AuditLedgerEntry.where(source_table:, source_id:))
    end
  end

  def create_live_tail_checkpoint(entry, entry_hash:)
    AuditCheckpoint.create!(
      household_id: entry.household_id,
      chain_key: entry.chain_key,
      chain_epoch: entry.chain_epoch,
      checkpoint_kind: 'periodic',
      sequence: entry.sequence,
      entry_hash:
    )
  end

  def install_repair_checkpoint_failure
    connection.execute <<~SQL
      CREATE FUNCTION spec_fail_legacy_repair_checkpoint() RETURNS trigger
      LANGUAGE plpgsql
      AS $$
      BEGIN
        IF NEW.checkpoint_kind = 'legacy-repair' THEN
          IF NOT EXISTS (
            SELECT 1
            FROM audit_ledger_entries entries
            JOIN audit_export_deliveries deliveries ON deliveries.audit_ledger_entry_id = entries.id
            WHERE entries.chain_epoch = NEW.chain_epoch
              AND entries.epoch_kind = 'legacy-repair'
              AND deliveries.status = 'pending'
          ) THEN
            RAISE EXCEPTION 'repair checkpoint reached before repaired entry and delivery';
          END IF;
          RAISE EXCEPTION 'injected failure after repaired entry and delivery';
        END IF;
        RETURN NEW;
      END;
      $$;

      CREATE TRIGGER spec_fail_legacy_repair_checkpoint
      BEFORE INSERT ON audit_checkpoints
      FOR EACH ROW EXECUTE FUNCTION spec_fail_legacy_repair_checkpoint();
    SQL
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

  def as_restricted_shared_login
    quoted_role = connection.quote_table_name(SHARED_LOGIN)
    connection.execute("CREATE ROLE #{quoted_role} LOGIN NOSUPERUSER NOBYPASSRLS INHERIT")
    connection.execute("GRANT med_tracker_owner TO #{quoted_role} WITH INHERIT TRUE, SET TRUE")
    connection.execute("SET LOCAL SESSION AUTHORIZATION #{connection.quote(SHARED_LOGIN)}")
    yield
  ensure
    connection.execute('RESET SESSION AUTHORIZATION')
    connection.execute("DROP ROLE IF EXISTS #{connection.quote_table_name(SHARED_LOGIN)}")
  end

  def expect_restricted_shared_login
    expect(connection.select_value('SELECT current_user')).to eq(SHARED_LOGIN)
    expect(connection.select_value('SELECT session_user')).to eq(SHARED_LOGIN)
    expect(connection.select_value(<<~SQL.squish)).to be(true)
      SELECT rolcanlogin AND NOT rolsuper AND NOT rolbypassrls
      FROM pg_roles
      WHERE rolname = current_user
    SQL
    expect(connection.select_value(<<~SQL.squish)).to be(true)
      SELECT pg_has_role(current_user, 'med_tracker_owner', 'MEMBER')
    SQL
  end
end
