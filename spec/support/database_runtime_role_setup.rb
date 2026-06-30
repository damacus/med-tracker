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
  ].freeze

  def self.call
    load_migrations
    apply_runtime_database_state
  end

  def self.load_migrations
    MIGRATIONS.each { |path| load Rails.root.join(path) }
  end

  def self.apply_runtime_database_state
    ActiveRecord::Migration.suppress_messages do
      CreateMedTrackerContextFunctions.new.up
      EnableHouseholdRowLevelSecurity.new.up
      AllowAccountScopedMembershipRlsBootstrap.new.up
      PartitionPaperTrailVersionsByHousehold.new.send(:enable_versions_rls)
      PartitionActiveStorageAttachmentsByHousehold.new.send(:enable_attachment_rls)
      ConfigureDatabaseRuntimeRoles.new.up
      AllowAccountLinkedPersonRlsLoginLookup.new.up
    end
  end
end

RSpec.configure do |config|
  config.before(:suite) do
    DatabaseRuntimeRoleSetup.call
  end
end
