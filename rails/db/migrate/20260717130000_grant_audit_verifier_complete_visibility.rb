# frozen_string_literal: true

class GrantAuditVerifierCompleteVisibility < ActiveRecord::Migration[8.1]
  VERIFIER_ROLE = 'med_tracker_audit_verifier'
  POLICY_NAME = 'audit_verifier_complete_visibility'
  AUDIT_TABLES = %w[
    audit_chain_heads audit_ledger_entries audit_export_deliveries audit_signing_keys audit_checkpoints
    versions security_audit_events
  ].freeze
  AUDIT_FUNCTIONS = [
    'audit_append_ledger_entry(text, bigint, bigint, jsonb, timestamptz)',
    'audit_capture_source_row()',
    'audit_record_signed_checkpoint(text, text, text, uuid, bigint, text, text, timestamptz, text)'
  ].freeze

  def up
    return unless role_exists?

    revoke_mutation_privileges
    execute "GRANT SELECT ON TABLE #{AUDIT_TABLES.join(', ')} TO #{VERIFIER_ROLE};"
    execute "DROP POLICY IF EXISTS #{POLICY_NAME} ON security_audit_events;"
    execute <<~SQL
      CREATE POLICY #{POLICY_NAME} ON security_audit_events
      FOR SELECT TO #{VERIFIER_ROLE}
      USING (true);
    SQL
  end

  def down
    execute "DROP POLICY IF EXISTS #{POLICY_NAME} ON security_audit_events;"
  end

  private

  def role_exists?
    select_value(<<~SQL.squish).to_i.positive?
      SELECT COUNT(*) FROM pg_roles WHERE rolname = #{quote(VERIFIER_ROLE)}
    SQL
  end

  def revoke_mutation_privileges
    execute <<~SQL
      REVOKE INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER
      ON TABLE #{AUDIT_TABLES.join(', ')}
      FROM #{VERIFIER_ROLE};
    SQL
    execute "REVOKE ALL PRIVILEGES ON SEQUENCE security_audit_events_id_seq FROM #{VERIFIER_ROLE};"
    execute "REVOKE ALL ON FUNCTION #{AUDIT_FUNCTIONS.join(', ')} FROM PUBLIC, #{VERIFIER_ROLE};"
  end
end
