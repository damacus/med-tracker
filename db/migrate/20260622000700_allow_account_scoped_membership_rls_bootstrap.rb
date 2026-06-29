# frozen_string_literal: true

class AllowAccountScopedMembershipRlsBootstrap < ActiveRecord::Migration[8.1]
  def up
    return unless household_memberships_ready?

    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON household_memberships;'
    execute <<~SQL
      CREATE POLICY household_tenant_isolation ON household_memberships
      USING (
        household_id = med_tracker.current_household_id()
        OR account_id = med_tracker.current_account_id()
      )
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end

  def down
    return unless household_memberships_ready?

    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON household_memberships;'
    execute <<~SQL
      CREATE POLICY household_tenant_isolation ON household_memberships
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end

  private

  def household_memberships_ready?
    table_exists?(:household_memberships) && column_exists?(:household_memberships, :household_id)
  end
end
