# frozen_string_literal: true

class ConfigureAuditVerifierRole < ActiveRecord::Migration[8.1]
  VERIFIER_ROLE = 'med_tracker_audit_verifier'
  AUDIT_TABLES = %w[
    audit_chain_heads audit_ledger_entries audit_export_deliveries audit_signing_keys audit_checkpoints
  ].freeze

  def up
    install_verifier_role
    lock_verifier_privileges
  end

  def down
    revoke_verifier_privileges
  end

  def install_verifier_role
    return if role_exists?(VERIFIER_ROLE)

    raise ActiveRecord::IrreversibleMigration, verifier_bootstrap_message unless can_create_roles?

    execute "CREATE ROLE #{VERIFIER_ROLE} NOLOGIN NOSUPERUSER NOBYPASSRLS;"
  end

  def lock_verifier_privileges
    revoke_verifier_privileges
    audit_objects = (AUDIT_TABLES + %w[versions security_audit_events]).join(', ')
    execute "GRANT USAGE ON SCHEMA public TO #{VERIFIER_ROLE};"
    execute "GRANT SELECT ON TABLE #{audit_objects} TO #{VERIFIER_ROLE};"
    execute "GRANT INSERT ON TABLE security_audit_events TO #{VERIFIER_ROLE};"
    execute "GRANT USAGE, SELECT ON SEQUENCE security_audit_events_id_seq TO #{VERIFIER_ROLE};"
  end

  def revoke_verifier_privileges
    audit_objects = (AUDIT_TABLES + %w[versions security_audit_events]).join(', ')
    execute "REVOKE ALL PRIVILEGES ON TABLE #{audit_objects} FROM #{VERIFIER_ROLE};"
    execute "REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM #{VERIFIER_ROLE};" if superuser?
    execute "REVOKE USAGE ON SCHEMA public FROM #{VERIFIER_ROLE};"
  end

  private

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

  def superuser?
    select_value(<<~SQL.squish)
      SELECT COALESCE(rolsuper, false)
      FROM pg_roles
      WHERE rolname = current_user
    SQL
  end

  def verifier_bootstrap_message
    'Database role med_tracker_audit_verifier is missing. Create it as NOLOGIN, NOSUPERUSER, NOBYPASSRLS before migrating.'
  end
end
