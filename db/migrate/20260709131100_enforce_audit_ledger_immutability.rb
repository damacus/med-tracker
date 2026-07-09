# frozen_string_literal: true

class EnforceAuditLedgerImmutability < ActiveRecord::Migration[8.1]
  LEDGER_TABLES = %w[
    audit_chain_heads audit_ledger_entries audit_export_deliveries audit_signing_keys audit_checkpoints
  ].freeze

  def up
    execute 'SET ROLE med_tracker_owner;'
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
    execute 'GRANT SELECT ON household_audit_ledger_entries TO med_tracker_app;'
    execute 'RESET ROLE;'
  end

  def down
    execute 'SET ROLE med_tracker_owner;'
    execute 'GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE versions, security_audit_events TO med_tracker_app;'
    LEDGER_TABLES.each do |table_name|
      execute "GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE #{table_name} TO med_tracker_app;"
      execute "GRANT USAGE, SELECT ON SEQUENCE #{table_name}_id_seq TO med_tracker_app;"
    end
    execute 'RESET ROLE;'
  end
end
