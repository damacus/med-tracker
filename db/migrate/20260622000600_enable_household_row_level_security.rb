# frozen_string_literal: true

class EnableHouseholdRowLevelSecurity < ActiveRecord::Migration[8.1]
  TENANT_TABLES = %w[
    people
    locations
    location_memberships
    medications
    medication_dosage_options
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
  ].freeze

  def up
    create_runtime_roles
    grant_runtime_role_access
    enable_rls_policies
  end

  def down
    TENANT_TABLES.each do |table_name|
      next unless household_table?(table_name)

      execute "DROP POLICY IF EXISTS household_tenant_isolation ON #{quote_table_name(table_name)};"
      execute "ALTER TABLE #{quote_table_name(table_name)} NO FORCE ROW LEVEL SECURITY;"
      execute "ALTER TABLE #{quote_table_name(table_name)} DISABLE ROW LEVEL SECURITY;"
    end
  end

  private

  def create_runtime_roles
    execute <<~SQL
      DO $$
      BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_owner') THEN
          CREATE ROLE med_tracker_owner NOLOGIN;
        END IF;

        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_app') THEN
          CREATE ROLE med_tracker_app NOLOGIN NOSUPERUSER NOBYPASSRLS;
        END IF;
      END
      $$;
    SQL
  end

  def grant_runtime_role_access
    execute 'GRANT USAGE ON SCHEMA public TO med_tracker_app;'
    execute 'GRANT USAGE ON SCHEMA med_tracker TO med_tracker_app;'
    execute 'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO med_tracker_app;'
    execute 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO med_tracker_app;'
  end

  def enable_rls_policies
    TENANT_TABLES.each do |table_name|
      next unless household_table?(table_name)

      quoted_table = quote_table_name(table_name)
      execute "ALTER TABLE #{quoted_table} ENABLE ROW LEVEL SECURITY;"
      execute "ALTER TABLE #{quoted_table} FORCE ROW LEVEL SECURITY;"
      execute "DROP POLICY IF EXISTS household_tenant_isolation ON #{quoted_table};"
      execute household_policy_sql(table_name, quoted_table)
    end
  end

  def household_policy_sql(table_name, quoted_table)
    return household_membership_policy_sql(quoted_table) if table_name == 'household_memberships'

    <<~SQL
      CREATE POLICY household_tenant_isolation ON #{quoted_table}
      USING (
        household_id IS NULL
        OR household_id = med_tracker.current_household_id()
      )
      WITH CHECK (
        household_id IS NULL
        OR household_id = med_tracker.current_household_id()
      );
    SQL
  end

  def household_membership_policy_sql(quoted_table)
    <<~SQL
      CREATE POLICY household_tenant_isolation ON #{quoted_table}
      USING (
        household_id IS NULL
        OR household_id = med_tracker.current_household_id()
        OR account_id = med_tracker.current_account_id()
      )
      WITH CHECK (
        household_id IS NULL
        OR household_id = med_tracker.current_household_id()
        OR account_id = med_tracker.current_account_id()
      );
    SQL
  end

  def household_table?(table_name)
    table_exists?(table_name) && column_exists?(table_name, :household_id)
  end
end
