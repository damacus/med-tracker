# frozen_string_literal: true

class EnforceRecentHouseholdRowLevelSecurity < ActiveRecord::Migration[8.1]
  TABLES = %w[
    health_events health_event_medications notification_events
    api_change_events api_idempotency_keys api_tombstones
  ].freeze

  def up
    install_policies
  end

  def down
    TABLES.each do |table_name|
      execute "DROP POLICY IF EXISTS household_tenant_isolation ON #{quote_table_name(table_name)};"
    end
  end

  def install_policies
    TABLES.each { |table_name| install_policy(table_name) }
  end

  private

  def install_policy(table_name)
    quoted_table = quote_table_name(table_name)
    execute "ALTER TABLE #{quoted_table} ENABLE ROW LEVEL SECURITY;"
    execute "ALTER TABLE #{quoted_table} FORCE ROW LEVEL SECURITY;"
    execute "DROP POLICY IF EXISTS household_tenant_isolation ON #{quoted_table};"
    execute <<~SQL.squish
      CREATE POLICY household_tenant_isolation ON #{quoted_table}
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end
end
