class RefreshDatabaseRuntimePrivileges < ActiveRecord::Migration[8.1]
  LEDGER_TABLES = %w[
    audit_chain_heads audit_ledger_entries audit_export_deliveries audit_signing_keys audit_checkpoints
  ].freeze

  def up
    execute 'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO med_tracker_app;'
    execute 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO med_tracker_app;'
    execute 'GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;'
    execute 'GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;'
    execute <<~SQL
      REVOKE INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER
      ON TABLE schema_migrations, ar_internal_metadata
      FROM med_tracker_app;
    SQL
    execute 'GRANT SELECT ON TABLE schema_migrations, ar_internal_metadata TO med_tracker_app;'
    execute <<~SQL
      ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public
      GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO med_tracker_app;
    SQL
    execute <<~SQL
      ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public
      GRANT USAGE, SELECT ON SEQUENCES TO med_tracker_app;
    SQL
    execute <<~SQL
      ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA med_tracker
      GRANT EXECUTE ON FUNCTIONS TO med_tracker_app;
    SQL
    enforce_audit_ledger_immutability
  end

  def down
    up
  end

  private

  def enforce_audit_ledger_immutability
    execute 'SET LOCAL ROLE med_tracker_owner;'
    execute <<~SQL.squish
      REVOKE UPDATE, DELETE ON TABLE versions, security_audit_events
      FROM med_tracker_app GRANTED BY med_tracker_owner;
    SQL
    LEDGER_TABLES.each do |table_name|
      execute <<~SQL.squish
        REVOKE ALL PRIVILEGES ON TABLE #{table_name}
        FROM med_tracker_app GRANTED BY med_tracker_owner;
      SQL
      execute <<~SQL.squish
        REVOKE ALL PRIVILEGES ON SEQUENCE #{table_name}_id_seq
        FROM med_tracker_app GRANTED BY med_tracker_owner;
      SQL
    end
    execute 'REVOKE ALL PRIVILEGES ON TABLE versions, security_audit_events FROM PUBLIC;'
    execute 'GRANT SELECT, INSERT ON TABLE versions, security_audit_events TO med_tracker_app;'
    execute 'GRANT USAGE, SELECT ON SEQUENCE versions_id_seq, security_audit_events_id_seq TO med_tracker_app;'
    execute <<~SQL
      DO $$
      BEGIN
        IF to_regclass('public.household_audit_ledger_entries') IS NOT NULL THEN
          EXECUTE 'REVOKE INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER
                   ON TABLE household_audit_ledger_entries FROM med_tracker_app GRANTED BY med_tracker_owner;';
          EXECUTE 'GRANT SELECT ON household_audit_ledger_entries TO med_tracker_app;';
        END IF;
      END
      $$;
    SQL
  end
end
