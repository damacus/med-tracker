# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260709131100_enforce_audit_ledger_immutability')
require Rails.root.join('db/migrate/20260715090000_refresh_database_runtime_privileges')

RSpec.describe 'ConfigureDatabaseRuntimeRoles' do
  it 'raises a pre-0.5 bootstrap error when runtime roles are missing and cannot be created' do
    migration = ConfigureDatabaseRuntimeRoles.new

    allow(migration).to receive_messages(runtime_role_exists?: false, can_create_roles?: false)

    expect { migration.send(:ensure_runtime_roles_bootstrapped!) }
      .to raise_error(ActiveRecord::IrreversibleMigration, /pre-0.5 database upgrade/)
  end

  it 'does not widen a pre-provisioned migration login' do
    migration = ConfigureDatabaseRuntimeRoles.new

    allow(migration).to receive(:can_create_roles?).and_return(false)
    allow(migration).to receive(:execute)

    migration.send(:grant_roles_to_login)

    expect(migration).not_to have_received(:execute)
  end

  describe RefreshDatabaseRuntimePrivileges do
    it 'refreshes runtime data access without allowing migration metadata writes' do
      migration = described_class.new
      statements = []

      allow(migration).to receive(:execute) { |sql| statements << sql.squish }

      migration.up

      expect(statements).to include(*expected_refresh_statements)
    end

    it 'preserves the final audit privilege contract after refreshing runtime privileges' do
      described_class.new.up

      expect_audit_privilege_contract
    ensure
      EnforceAuditLedgerImmutability.new.up
    end

    def expected_refresh_statements
      [
        'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO med_tracker_app;',
        'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO med_tracker_app;',
        'GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;',
        'GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;',
        'REVOKE INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER ' \
        'ON TABLE schema_migrations, ar_internal_metadata FROM med_tracker_app;',
        'GRANT SELECT ON TABLE schema_migrations, ar_internal_metadata TO med_tracker_app;',
        'ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public ' \
        'GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO med_tracker_app;',
        'ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public ' \
        'GRANT USAGE, SELECT ON SEQUENCES TO med_tracker_app;',
        'ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA med_tracker ' \
        'GRANT EXECUTE ON FUNCTIONS TO med_tracker_app;'
      ]
    end

    def expect_audit_privilege_contract
      expect_source_audit_privileges
      expect_household_audit_view_privilege
      expect_no_ledger_privileges
    end

    def expect_source_audit_privileges
      %w[versions security_audit_events].each do |table_name|
        expect(table_privilege(table_name, 'SELECT')).to be(true)
        expect(table_privilege(table_name, 'INSERT')).to be(true)
        expect(table_privilege(table_name, 'UPDATE')).to be(false)
        expect(table_privilege(table_name, 'DELETE')).to be(false)
      end
    end

    def expect_household_audit_view_privilege
      expect(table_privilege('household_audit_ledger_entries', 'SELECT')).to be(true)
      %w[INSERT UPDATE DELETE].each do |action|
        expect(table_privilege('household_audit_ledger_entries', action)).to be(false)
      end
    end

    def expect_no_ledger_privileges
      EnforceAuditLedgerImmutability::LEDGER_TABLES.each do |table_name|
        %w[SELECT INSERT UPDATE DELETE].each do |action|
          expect(table_privilege(table_name, action)).to be(false)
        end
        expect(sequence_privilege("#{table_name}_id_seq", 'USAGE')).to be(false)
        expect(sequence_privilege("#{table_name}_id_seq", 'SELECT')).to be(false)
      end
    end

    def table_privilege(table_name, action)
      ActiveRecord::Base.connection.select_value(
        "SELECT has_table_privilege('med_tracker_app', '#{table_name}', '#{action}')"
      )
    end

    def sequence_privilege(sequence_name, action)
      ActiveRecord::Base.connection.select_value(
        "SELECT has_sequence_privilege('med_tracker_app', '#{sequence_name}', '#{action}')"
      )
    end
  end
end
