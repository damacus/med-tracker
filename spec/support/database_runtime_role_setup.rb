# frozen_string_literal: true

module DatabaseRuntimeRoleSetup
  MIGRATIONS = %w[
    db/migrate/20260622000200_create_med_tracker_context_functions.rb
    db/migrate/20260622000600_enable_household_row_level_security.rb
    db/migrate/20260622000700_allow_account_scoped_membership_rls_bootstrap.rb
    db/migrate/20260622000800_partition_paper_trail_versions_by_household.rb
    db/migrate/20260622000900_partition_active_storage_attachments_by_household.rb
    db/migrate/20260622001000_configure_database_runtime_roles.rb
    db/migrate/20260630141000_allow_account_linked_person_rls_login_lookup.rb
    db/migrate/20260709131000_create_tamper_evident_audit_ledger.rb
    db/migrate/20260709131100_enforce_audit_ledger_immutability.rb
    db/migrate/20260709143000_configure_audit_object_lock_exporter.rb
    db/migrate/20260709143100_enforce_recent_household_row_level_security.rb
    db/migrate/20260709150000_configure_audit_verifier_role.rb
    db/migrate/20260713190100_allow_invitation_token_rls_bootstrap.rb
  ].freeze

  def self.call
    load_migrations
    apply_runtime_database_state
  end

  def self.load_migrations
    MIGRATIONS.each do |path|
      class_name = File.basename(path, '.rb').sub(/^\d+_/, '').camelize
      load Rails.root.join(path) unless Object.const_defined?(class_name)
    end
  end

  def self.apply_runtime_database_state
    ActiveRecord::Migration.suppress_messages do
      apply_tenant_database_state
      install_audit_ledger_database_objects
      EnforceAuditLedgerImmutability.new.up
      install_audit_exporter_database_objects
      EnforceRecentHouseholdRowLevelSecurity.new.install_policies
      install_audit_verifier_database_objects
    end
  end

  def self.apply_tenant_database_state
    CreateMedTrackerContextFunctions.new.up
    EnableHouseholdRowLevelSecurity.new.up
    AllowAccountScopedMembershipRlsBootstrap.new.up
    PartitionPaperTrailVersionsByHousehold.new.send(:enable_versions_rls)
    PartitionActiveStorageAttachmentsByHousehold.new.send(:enable_attachment_rls)
    ConfigureDatabaseRuntimeRoles.new.up
    AllowAccountLinkedPersonRlsLoginLookup.new.up
    AllowInvitationTokenRlsBootstrap.new.up
  end

  def self.install_audit_ledger_database_objects
    migration = CreateTamperEvidentAuditLedger.new
    migration.send(:install_ledger_functions)
    migration.send(:install_source_triggers)
    migration.send(:create_household_read_view)
  end

  def self.install_audit_exporter_database_objects
    migration = ConfigureAuditObjectLockExporter.new
    migration.send(:install_exporter_role)
    migration.send(:install_checkpoint_function)
    migration.send(:lock_exporter_privileges)
  end

  def self.install_audit_verifier_database_objects
    migration = ConfigureAuditVerifierRole.new
    migration.install_verifier_role
    migration.lock_verifier_privileges
  end
end

RSpec.configure do |config|
  config.before(:suite) do
    DatabaseRuntimeRoleSetup.call
  end
end
