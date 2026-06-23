# frozen_string_literal: true

class PartitionPaperTrailVersionsByHousehold < ActiveRecord::Migration[8.1]
  def change
    add_reference :versions, :household, foreign_key: true, index: true
    add_reference :versions, :actor_membership, foreign_key: { to_table: :household_memberships }, index: true

    reversible do |dir|
      dir.up { enable_versions_rls }
      dir.down { disable_versions_rls }
    end
  end

  private

  def enable_versions_rls
    execute 'ALTER TABLE versions ENABLE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE versions FORCE ROW LEVEL SECURITY;'
    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON versions;'
    execute <<~SQL
      CREATE POLICY household_tenant_isolation ON versions
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

  def disable_versions_rls
    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON versions;'
    execute 'ALTER TABLE versions NO FORCE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE versions DISABLE ROW LEVEL SECURITY;'
  end
end
