# frozen_string_literal: true

class CreateTamperEvidentAuditLedger < ActiveRecord::Migration[8.1]
  AUDIT_TABLES = %w[
    audit_chain_heads audit_ledger_entries audit_export_deliveries audit_signing_keys audit_checkpoints
  ].freeze

  def up
    enable_extension 'pgcrypto' unless extension_enabled?('pgcrypto')
    create_audit_tables
    transfer_audit_ownership
    create_constraints
    install_ledger_functions
    install_source_triggers
    backfill_legacy_evidence
    create_household_read_view
    lock_audit_privileges
  end

  def down
    execute 'DROP VIEW IF EXISTS household_audit_ledger_entries;'
    execute 'DROP TRIGGER IF EXISTS append_versions_to_audit_ledger ON versions;'
    execute 'DROP TRIGGER IF EXISTS append_security_events_to_audit_ledger ON security_audit_events;'
    execute 'DROP FUNCTION IF EXISTS audit_capture_source_row();'
    execute 'DROP FUNCTION IF EXISTS audit_append_ledger_entry(text, bigint, bigint, jsonb, timestamptz);'
    AUDIT_TABLES.reverse_each { |table_name| drop_table table_name, if_exists: true }
  end

  private

  def create_audit_tables
    create_table :audit_chain_heads do |t|
      t.string :chain_key, null: false
      t.references :household, foreign_key: true
      t.uuid :chain_epoch, null: false, default: -> { 'gen_random_uuid()' }
      t.string :epoch_kind, null: false, default: 'live'
      t.bigint :last_sequence, null: false, default: 0
      t.binary :last_hash
      t.timestamps
    end
    add_index :audit_chain_heads, :chain_key, unique: true

    create_table :audit_ledger_entries do |t|
      t.references :household, foreign_key: true
      t.string :chain_key, null: false
      t.uuid :chain_epoch, null: false
      t.string :epoch_kind, null: false
      t.bigint :sequence, null: false
      t.binary :previous_hash
      t.binary :entry_hash, null: false
      t.string :hash_algorithm, null: false, default: 'sha256'
      t.integer :schema_version, null: false, default: 1
      t.string :source_table, null: false
      t.bigint :source_id, null: false
      t.jsonb :source_payload, null: false
      t.jsonb :envelope, null: false
      t.binary :canonical_payload, null: false
      t.datetime :occurred_at, null: false
      t.string :retention_policy_version, null: false
      t.datetime :retain_until, null: false
      t.timestamps
    end
    add_index :audit_ledger_entries, %i[source_table source_id], unique: true
    add_index :audit_ledger_entries, %i[chain_key chain_epoch sequence], unique: true,
                                                                    name: 'idx_audit_ledger_chain_sequence'
    add_index :audit_ledger_entries, %i[household_id occurred_at]
    add_index :audit_ledger_entries, :retain_until

    create_table :audit_export_deliveries do |t|
      t.references :audit_ledger_entry, null: false, foreign_key: true, index: { unique: true }
      t.string :status, null: false, default: 'pending'
      t.integer :attempts, null: false, default: 0
      t.string :object_key
      t.string :checksum_sha256
      t.string :object_version_id
      t.string :retention_mode
      t.datetime :retain_until
      t.datetime :next_attempt_at
      t.datetime :delivered_at
      t.string :last_error_code
      t.text :last_error_message
      t.timestamps
    end
    add_index :audit_export_deliveries, %i[status next_attempt_at]

    create_table :audit_signing_keys do |t|
      t.string :key_id, null: false
      t.string :algorithm, null: false, default: 'ed25519'
      t.binary :public_key, null: false
      t.datetime :active_from, null: false
      t.datetime :retired_at
      t.timestamps
    end
    add_index :audit_signing_keys, :key_id, unique: true

    create_table :audit_checkpoints do |t|
      t.references :household, foreign_key: true
      t.references :audit_signing_key, foreign_key: true
      t.string :chain_key, null: false
      t.uuid :chain_epoch, null: false
      t.string :checkpoint_kind, null: false, default: 'periodic'
      t.bigint :sequence, null: false
      t.binary :entry_hash, null: false
      t.binary :signature
      t.datetime :signed_at
      t.timestamps
    end
    add_index :audit_checkpoints, %i[chain_key chain_epoch sequence], unique: true,
                                                                  name: 'idx_audit_checkpoint_chain_sequence'
  end

  def create_constraints
    execute <<~SQL
      ALTER TABLE audit_chain_heads
        ADD CONSTRAINT audit_chain_heads_last_hash_length
        CHECK (last_hash IS NULL OR octet_length(last_hash) = 32);
      ALTER TABLE audit_ledger_entries
        ADD CONSTRAINT audit_ledger_previous_hash_length
        CHECK (previous_hash IS NULL OR octet_length(previous_hash) = 32),
        ADD CONSTRAINT audit_ledger_entry_hash_length
        CHECK (octet_length(entry_hash) = 32),
        ADD CONSTRAINT audit_ledger_positive_sequence
        CHECK (sequence > 0),
        ADD CONSTRAINT audit_ledger_retention_after_event
        CHECK (retain_until >= occurred_at);
      ALTER TABLE audit_checkpoints
        ADD CONSTRAINT audit_checkpoint_entry_hash_length
        CHECK (octet_length(entry_hash) = 32);
    SQL
  end

  def transfer_audit_ownership
    (AUDIT_TABLES + %w[versions security_audit_events]).each do |table_name|
      execute "ALTER TABLE #{table_name} OWNER TO med_tracker_owner;"
    end
  end

  def install_ledger_functions
    execute <<~SQL
      CREATE OR REPLACE FUNCTION audit_append_ledger_entry(
        p_source_table text,
        p_source_id bigint,
        p_household_id bigint,
        p_source_payload jsonb,
        p_occurred_at timestamptz
      ) RETURNS void
      LANGUAGE plpgsql
      SECURITY DEFINER
      SET search_path = pg_catalog, public
      AS $$
      DECLARE
        v_chain_key text := COALESCE('household:' || p_household_id::text, 'global');
        v_head audit_chain_heads%ROWTYPE;
        v_sequence bigint;
        v_context jsonb := COALESCE(p_source_payload->'audit_context', '{}'::jsonb);
        v_metadata jsonb := COALESCE(p_source_payload->'metadata', '{}'::jsonb);
        v_occurred_at timestamptz := COALESCE(p_occurred_at, clock_timestamp());
        v_retain_until timestamptz := v_occurred_at + interval '10 years';
        v_policy_version text := COALESCE(v_context->>'retention_policy_version', 'clinical-security-v1');
        v_envelope jsonb;
        v_canonical_payload bytea;
        v_entry_hash bytea;
      BEGIN
        INSERT INTO audit_chain_heads (chain_key, household_id, created_at, updated_at)
        VALUES (v_chain_key, p_household_id, clock_timestamp(), clock_timestamp())
        ON CONFLICT (chain_key) DO NOTHING;

        SELECT * INTO STRICT v_head
        FROM audit_chain_heads
        WHERE chain_key = v_chain_key
        FOR UPDATE;

        v_sequence := v_head.last_sequence + 1;
        v_envelope := jsonb_strip_nulls(jsonb_build_object(
          'schema_version', 1,
          'event_id', gen_random_uuid(),
          'event_type', COALESCE(p_source_payload->>'event_type', p_source_payload->>'event'),
          'outcome', v_metadata->>'outcome',
          'occurred_at', v_occurred_at,
          'household_id', p_household_id,
          'agent', jsonb_build_object(
            'account_id', v_context->'actor_account_id',
            'user_id', v_context->'actor_user_id',
            'membership_id', v_context->'actor_membership_id',
            'role', v_context->'active_role',
            'permissions_version', v_context->'permissions_version',
            'authentication_method', v_context->'authentication_method',
            'session_reference', v_context->'session_reference'
          ),
          'policy', jsonb_build_object('class', v_context->'policy_class', 'query', v_context->'policy_query'),
          'request', jsonb_build_object(
            'request_id', v_context->'request_id',
            'ip', v_context->'ip',
            'support_access_session_id', v_context->'support_access_session_id'
          ),
          'source', jsonb_build_object('table', p_source_table, 'id', p_source_id),
          'entity', CASE WHEN p_source_table = 'versions' THEN
            jsonb_build_object('type', p_source_payload->'item_type', 'id', p_source_payload->'item_id')
          ELSE v_metadata - 'outcome' END,
          'retention', jsonb_build_object('policy_version', v_policy_version, 'retain_until', v_retain_until)
        ));
        v_canonical_payload := convert_to(v_envelope::text, 'UTF8');
        v_entry_hash := digest(
          convert_to(
            'medtracker.audit.ledger.v1' || chr(31) || v_chain_key || chr(31) ||
            v_head.chain_epoch::text || chr(31) || v_sequence::text || chr(31),
            'UTF8'
          ) || COALESCE(v_head.last_hash, '\\x'::bytea) || v_canonical_payload,
          'sha256'
        );

        INSERT INTO audit_ledger_entries (
          household_id, chain_key, chain_epoch, epoch_kind, sequence, previous_hash, entry_hash,
          source_table, source_id, source_payload, envelope, canonical_payload, occurred_at,
          retention_policy_version, retain_until, created_at, updated_at
        ) VALUES (
          p_household_id, v_chain_key, v_head.chain_epoch, v_head.epoch_kind, v_sequence, v_head.last_hash,
          v_entry_hash, p_source_table, p_source_id, p_source_payload - 'updated_at', v_envelope,
          v_canonical_payload, v_occurred_at, v_policy_version, v_retain_until, clock_timestamp(), clock_timestamp()
        );

        INSERT INTO audit_export_deliveries (audit_ledger_entry_id, status, attempts, created_at, updated_at)
        VALUES (currval(pg_get_serial_sequence('audit_ledger_entries', 'id')), 'pending', 0,
                clock_timestamp(), clock_timestamp());

        UPDATE audit_chain_heads
        SET last_sequence = v_sequence, last_hash = v_entry_hash, updated_at = clock_timestamp()
        WHERE id = v_head.id;
      END;
      $$;

      CREATE OR REPLACE FUNCTION audit_capture_source_row() RETURNS trigger
      LANGUAGE plpgsql
      SECURITY DEFINER
      SET search_path = pg_catalog, public
      AS $$
      BEGIN
        PERFORM audit_append_ledger_entry(TG_TABLE_NAME, NEW.id, NEW.household_id, to_jsonb(NEW), NEW.created_at);
        RETURN NEW;
      END;
      $$;

      ALTER FUNCTION audit_append_ledger_entry(text, bigint, bigint, jsonb, timestamptz)
        OWNER TO med_tracker_owner;
      ALTER FUNCTION audit_capture_source_row() OWNER TO med_tracker_owner;
    SQL
  end

  def install_source_triggers
    execute <<~SQL
      DROP TRIGGER IF EXISTS append_versions_to_audit_ledger ON versions;
      CREATE TRIGGER append_versions_to_audit_ledger
      AFTER INSERT ON versions
      FOR EACH ROW EXECUTE FUNCTION audit_capture_source_row();

      DROP TRIGGER IF EXISTS append_security_events_to_audit_ledger ON security_audit_events;
      CREATE TRIGGER append_security_events_to_audit_ledger
      AFTER INSERT ON security_audit_events
      FOR EACH ROW EXECUTE FUNCTION audit_capture_source_row();
    SQL
  end

  def backfill_legacy_evidence
    execute <<~SQL
      INSERT INTO audit_chain_heads (chain_key, household_id, epoch_kind, created_at, updated_at)
      SELECT DISTINCT COALESCE('household:' || household_id::text, 'global'), household_id,
                      'legacy-baseline', clock_timestamp(), clock_timestamp()
      FROM (
        SELECT household_id FROM versions
        UNION ALL
        SELECT household_id FROM security_audit_events
      ) sources
      ON CONFLICT (chain_key) DO NOTHING;

      DO $$
      DECLARE source_row record;
      BEGIN
        FOR source_row IN
          SELECT source_table, source_id, household_id, source_payload, occurred_at
          FROM (
            SELECT 'versions'::text source_table, id source_id, household_id,
                   to_jsonb(versions.*) source_payload, created_at occurred_at
            FROM versions
            UNION ALL
            SELECT 'security_audit_events'::text, id, household_id,
                   to_jsonb(security_audit_events.*), created_at
            FROM security_audit_events
          ) sources
          WHERE NOT EXISTS (
            SELECT 1 FROM audit_ledger_entries entries
            WHERE entries.source_table = sources.source_table AND entries.source_id = sources.source_id
          )
          ORDER BY COALESCE('household:' || household_id::text, 'global'), occurred_at, source_table, source_id
        LOOP
          PERFORM audit_append_ledger_entry(
            source_row.source_table, source_row.source_id, source_row.household_id,
            source_row.source_payload, source_row.occurred_at
          );
        END LOOP;
      END
      $$;

      INSERT INTO audit_checkpoints (
        household_id, chain_key, chain_epoch, checkpoint_kind, sequence, entry_hash, created_at, updated_at
      )
      SELECT household_id, chain_key, chain_epoch, 'legacy-baseline', last_sequence, last_hash,
             clock_timestamp(), clock_timestamp()
      FROM audit_chain_heads
      WHERE epoch_kind = 'legacy-baseline' AND last_sequence > 0;

      UPDATE audit_chain_heads
      SET chain_epoch = gen_random_uuid(), epoch_kind = 'live', last_sequence = 0, last_hash = NULL,
          updated_at = clock_timestamp()
      WHERE epoch_kind = 'legacy-baseline';
    SQL
  end

  def create_household_read_view
    execute <<~SQL
      CREATE OR REPLACE VIEW household_audit_ledger_entries
      WITH (security_barrier = true)
      AS
      SELECT id, household_id, chain_key, chain_epoch, epoch_kind, sequence, previous_hash, entry_hash,
             hash_algorithm, schema_version, source_table, source_id, envelope, occurred_at,
             retention_policy_version, retain_until, created_at
      FROM audit_ledger_entries
      WHERE household_id = med_tracker.current_household_id();
      ALTER VIEW household_audit_ledger_entries OWNER TO med_tracker_owner;
    SQL
  end

  def lock_audit_privileges
    AUDIT_TABLES.each do |table_name|
      execute "REVOKE ALL ON TABLE #{table_name} FROM med_tracker_app;"
      execute "REVOKE ALL ON SEQUENCE #{table_name}_id_seq FROM med_tracker_app;"
    end
    execute 'REVOKE UPDATE, DELETE ON TABLE versions, security_audit_events FROM med_tracker_app;'
    execute 'GRANT SELECT, INSERT ON TABLE versions, security_audit_events TO med_tracker_app;'
    execute 'GRANT USAGE, SELECT ON SEQUENCE versions_id_seq, security_audit_events_id_seq TO med_tracker_app;'
    execute 'GRANT SELECT ON household_audit_ledger_entries TO med_tracker_app;'
    execute 'REVOKE ALL ON FUNCTION audit_append_ledger_entry(text, bigint, bigint, jsonb, timestamptz) FROM PUBLIC;'
    execute 'REVOKE ALL ON FUNCTION audit_capture_source_row() FROM PUBLIC;'
  end
end
