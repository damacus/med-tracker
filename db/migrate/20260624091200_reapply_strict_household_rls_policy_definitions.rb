# frozen_string_literal: true

class ReapplyStrictHouseholdRlsPolicyDefinitions < ActiveRecord::Migration[8.1]
  TENANT_TABLES = %w[
    people
    locations
    location_memberships
    medications
    dosages
    schedules
    person_medications
    medication_takes
    notification_preferences
    household_memberships
    person_access_grants
    household_invitations
    household_invitation_grants
    security_audit_events
    active_storage_attachments
  ].freeze

  def up
    TENANT_TABLES.each do |table_name|
      next unless household_table?(table_name)

      quoted_table = quote_table_name(table_name)
      execute "DROP POLICY IF EXISTS household_tenant_isolation ON #{quoted_table};"
      execute household_policy_sql(table_name, quoted_table)
    end

    disable_global_versions_rls
  end

  def down
    TENANT_TABLES.each do |table_name|
      execute "DROP POLICY IF EXISTS household_tenant_isolation ON #{quote_table_name(table_name)};" if household_table?(table_name)
    end
  end

  private

  def household_policy_sql(table_name, quoted_table)
    return household_membership_policy_sql(quoted_table) if table_name == 'household_memberships'

    <<~SQL
      CREATE POLICY household_tenant_isolation ON #{quoted_table}
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end

  def household_membership_policy_sql(quoted_table)
    <<~SQL
      CREATE POLICY household_tenant_isolation ON #{quoted_table}
      USING (
        household_id = med_tracker.current_household_id()
        OR account_id = med_tracker.current_account_id()
      )
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end

  def household_table?(table_name)
    table_exists?(table_name) && column_exists?(table_name, :household_id)
  end

  def disable_global_versions_rls
    return unless household_table?(:versions)

    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON versions;'
    execute 'ALTER TABLE versions NO FORCE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE versions DISABLE ROW LEVEL SECURITY;'
  end
end
