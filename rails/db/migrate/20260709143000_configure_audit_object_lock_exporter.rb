# frozen_string_literal: true

class ConfigureAuditObjectLockExporter < ActiveRecord::Migration[8.1]
  EXPORTER_ROLE = 'med_tracker_audit_exporter'

  def up
    add_checkpoint_deliveries
    install_exporter_role
    install_checkpoint_function
    lock_exporter_privileges
  end

  def down
    execute "REVOKE ALL ON FUNCTION audit_record_signed_checkpoint(text, text, text, uuid, bigint, text, text, timestamptz, text) FROM #{EXPORTER_ROLE};"
    execute 'DROP FUNCTION IF EXISTS audit_record_signed_checkpoint(text, text, text, uuid, bigint, text, text, timestamptz, text);'
    remove_check_constraint :audit_export_deliveries, name: 'audit_export_delivery_exactly_one_record'
    remove_reference :audit_export_deliveries, :audit_checkpoint, foreign_key: true
    change_column_null :audit_export_deliveries, :audit_ledger_entry_id, false
  end

  private

  def add_checkpoint_deliveries
    change_column_null :audit_export_deliveries, :audit_ledger_entry_id, true
    add_reference :audit_export_deliveries, :audit_checkpoint, foreign_key: true, index: { unique: true }
    add_check_constraint :audit_export_deliveries,
                         '(audit_ledger_entry_id IS NULL) <> (audit_checkpoint_id IS NULL)',
                         name: 'audit_export_delivery_exactly_one_record'
  end

  def install_exporter_role
    return if role_exists?(EXPORTER_ROLE)

    raise ActiveRecord::IrreversibleMigration, exporter_bootstrap_message unless can_create_roles?

    execute "CREATE ROLE #{EXPORTER_ROLE} NOLOGIN NOSUPERUSER NOBYPASSRLS;"
  end

  def install_checkpoint_function
    execute <<~SQL
      CREATE OR REPLACE FUNCTION audit_record_signed_checkpoint(
        p_key_id text,
        p_public_key_base64 text,
        p_chain_key text,
        p_chain_epoch uuid,
        p_sequence bigint,
        p_entry_hash_hex text,
        p_signature_base64 text,
        p_signed_at timestamptz,
        p_checkpoint_kind text
      ) RETURNS bigint
      LANGUAGE plpgsql
      SECURITY DEFINER
      SET search_path = pg_catalog, public
      AS $$
      DECLARE
        v_entry audit_ledger_entries%ROWTYPE;
        v_key audit_signing_keys%ROWTYPE;
        v_checkpoint audit_checkpoints%ROWTYPE;
      BEGIN
        SELECT * INTO STRICT v_entry
        FROM audit_ledger_entries
        WHERE chain_key = p_chain_key
          AND chain_epoch = p_chain_epoch
          AND sequence = p_sequence
          AND entry_hash = decode(p_entry_hash_hex, 'hex');

        INSERT INTO audit_signing_keys (key_id, algorithm, public_key, active_from, created_at, updated_at)
        VALUES (p_key_id, 'ed25519', decode(p_public_key_base64, 'base64'), p_signed_at,
                clock_timestamp(), clock_timestamp())
        ON CONFLICT (key_id) DO NOTHING;

        SELECT * INTO STRICT v_key FROM audit_signing_keys WHERE key_id = p_key_id;
        IF v_key.public_key <> decode(p_public_key_base64, 'base64') THEN
          RAISE EXCEPTION 'audit signing key id is already registered with different key material';
        END IF;

        SELECT * INTO v_checkpoint
        FROM audit_checkpoints
        WHERE chain_key = p_chain_key AND chain_epoch = p_chain_epoch AND sequence = p_sequence
        FOR UPDATE;

        IF FOUND AND v_checkpoint.signature IS NOT NULL THEN
          RAISE EXCEPTION 'audit checkpoint is already signed';
        END IF;

        IF FOUND THEN
          UPDATE audit_checkpoints
          SET audit_signing_key_id = v_key.id,
              signature = decode(p_signature_base64, 'base64'),
              signed_at = p_signed_at,
              updated_at = clock_timestamp()
          WHERE id = v_checkpoint.id
          RETURNING * INTO v_checkpoint;
        ELSE
          INSERT INTO audit_checkpoints (
            household_id, audit_signing_key_id, chain_key, chain_epoch, checkpoint_kind,
            sequence, entry_hash, signature, signed_at, created_at, updated_at
          ) VALUES (
            v_entry.household_id, v_key.id, p_chain_key, p_chain_epoch, p_checkpoint_kind,
            p_sequence, decode(p_entry_hash_hex, 'hex'), decode(p_signature_base64, 'base64'),
            p_signed_at, clock_timestamp(), clock_timestamp()
          ) RETURNING * INTO v_checkpoint;
        END IF;

        INSERT INTO audit_export_deliveries (
          audit_checkpoint_id, status, attempts, created_at, updated_at
        ) VALUES (
          v_checkpoint.id, 'pending', 0, clock_timestamp(), clock_timestamp()
        ) ON CONFLICT (audit_checkpoint_id) DO NOTHING;

        RETURN v_checkpoint.id;
      END;
      $$;

      ALTER FUNCTION audit_record_signed_checkpoint(text, text, text, uuid, bigint, text, text, timestamptz, text)
        OWNER TO med_tracker_owner;
      REVOKE ALL ON FUNCTION audit_record_signed_checkpoint(text, text, text, uuid, bigint, text, text, timestamptz, text)
        FROM PUBLIC;
    SQL
  end

  def lock_exporter_privileges
    execute <<~SQL.squish
      REVOKE ALL PRIVILEGES ON TABLE
      audit_chain_heads, audit_ledger_entries, audit_export_deliveries, audit_signing_keys, audit_checkpoints,
      versions, security_audit_events
      FROM #{EXPORTER_ROLE};
    SQL
    execute <<~SQL.squish
      REVOKE ALL PRIVILEGES ON SEQUENCE
      audit_chain_heads_id_seq, audit_ledger_entries_id_seq, audit_export_deliveries_id_seq,
      audit_signing_keys_id_seq, audit_checkpoints_id_seq
      FROM #{EXPORTER_ROLE};
    SQL
    execute "GRANT USAGE ON SCHEMA public TO #{EXPORTER_ROLE};"
    execute <<~SQL.squish
      GRANT SELECT ON TABLE audit_chain_heads, audit_ledger_entries, audit_checkpoints, audit_signing_keys
      TO #{EXPORTER_ROLE};
    SQL
    execute "GRANT SELECT, UPDATE ON TABLE audit_export_deliveries TO #{EXPORTER_ROLE};"
    execute <<~SQL.squish
      GRANT EXECUTE ON FUNCTION
      audit_record_signed_checkpoint(text, text, text, uuid, bigint, text, text, timestamptz, text)
      TO #{EXPORTER_ROLE};
    SQL
  end

  def role_exists?(role_name)
    select_value("SELECT COUNT(*) FROM pg_roles WHERE rolname = #{quote(role_name)}").to_i.positive?
  end

  def can_create_roles?
    select_value(<<~SQL.squish)
      SELECT COALESCE(rolsuper OR rolcreaterole, false)
      FROM pg_roles
      WHERE rolname = current_user
    SQL
  end

  def exporter_bootstrap_message
    'Database role med_tracker_audit_exporter is missing. Create it as NOLOGIN, NOSUPERUSER, NOBYPASSRLS before migrating.'
  end
end
