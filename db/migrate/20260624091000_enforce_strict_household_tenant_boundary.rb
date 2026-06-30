# frozen_string_literal: true

class EnforceStrictHouseholdTenantBoundary < ActiveRecord::Migration[8.1]
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
    relax_forced_rls_for_backfill
    begin
      backfill_derived_household_ids
      assert_no_null_household_ids!
      enforce_household_id_not_null
      replace_household_rls_policies
      disable_global_versions_rls
    ensure
      restore_forced_rls_after_backfill
    end
  end

  def down
    TENANT_TABLES.each do |table_name|
      change_column_null table_name, :household_id, true if household_table?(table_name)
    end
  end

  private

  def relax_forced_rls_for_backfill
    TENANT_TABLES.each do |table_name|
      execute "ALTER TABLE #{quote_table_name(table_name)} NO FORCE ROW LEVEL SECURITY;" if household_table?(table_name)
    end
  end

  def restore_forced_rls_after_backfill
    TENANT_TABLES.each do |table_name|
      execute "ALTER TABLE #{quote_table_name(table_name)} FORCE ROW LEVEL SECURITY;" if household_table?(table_name)
    end
  end

  def backfill_derived_household_ids
    backfill_root_household_ids
    update_from_parent(:location_memberships, :locations, :location_id)
    update_from_parent(:location_memberships, :people, :person_id)
    update_from_parent(:medications, :locations, :location_id)
    update_from_parent(:dosages, :medications, :medication_id)
    update_from_parent(:schedules, :people, :person_id)
    update_from_parent(:schedules, :medications, :medication_id)
    update_from_parent(:schedules, :dosages, :source_dosage_option_id)
    update_from_parent(:person_medications, :people, :person_id)
    update_from_parent(:person_medications, :medications, :medication_id)
    update_from_parent(:person_medications, :dosages, :source_dosage_option_id)
    update_from_parent(:medication_takes, :schedules, :schedule_id)
    update_from_parent(:medication_takes, :person_medications, :person_medication_id)
    update_from_parent(:medication_takes, :medications, :taken_from_medication_id)
    update_from_parent(:medication_takes, :locations, :taken_from_location_id)
    update_from_parent(:notification_preferences, :people, :person_id)
    update_from_parent(:household_invitation_grants, :household_invitations, :household_invitation_id)
    update_from_parent(:person_access_grants, :household_memberships, :household_membership_id)
    update_from_parent(:security_audit_events, :household_memberships, :actor_membership_id)
    backfill_active_storage_attachment_households
  end

  def backfill_root_household_ids
    return unless orphan_root_tenant_rows?

    household_id = root_backfill_household_id
    execute <<~SQL.squish if household_table?(:people)
      UPDATE people
      SET household_id = #{household_id}
      WHERE household_id IS NULL;
    SQL
    execute <<~SQL.squish if household_table?(:locations)
      UPDATE locations
      SET household_id = #{household_id}
      WHERE household_id IS NULL;
    SQL
  end

  def orphan_root_tenant_rows?
    %i[people locations].any? do |table_name|
      household_table?(table_name) &&
        select_value("SELECT COUNT(*) FROM #{quote_table_name(table_name)} WHERE household_id IS NULL").to_i.positive?
    end
  end

  def root_backfill_household_id
    household_count = select_value('SELECT COUNT(*) FROM households').to_i
    return select_value('SELECT id FROM households ORDER BY id LIMIT 1').to_i if household_count == 1
    return create_legacy_household if household_count.zero?

    raise ActiveRecord::IrreversibleMigration,
          'root tenant rows without household_id cannot be derived when multiple households exist'
  end

  def create_legacy_household
    select_value(<<~SQL.squish).to_i
      INSERT INTO households (name, slug, status, timezone, subscription_plan, created_at, updated_at)
      VALUES ('Legacy Household', 'legacy-household', 'active', 'UTC', 'free', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
      RETURNING id
    SQL
  end

  def update_from_parent(child_table, parent_table, foreign_key)
    return unless household_table?(child_table)
    return unless household_table?(parent_table)
    return unless column_exists?(child_table, foreign_key)

    child = quote_table_name(child_table)
    parent = quote_table_name(parent_table)
    quoted_foreign_key = quote_column_name(foreign_key)

    execute <<~SQL.squish
      UPDATE #{child}
      SET household_id = #{parent}.household_id
      FROM #{parent}
      WHERE #{child}.#{quoted_foreign_key} = #{parent}.id
        AND #{child}.household_id IS NULL
        AND #{parent}.household_id IS NOT NULL;
    SQL
  end

  def backfill_active_storage_attachment_households
    return unless household_table?(:active_storage_attachments)

    execute <<~SQL.squish
      UPDATE active_storage_attachments
      SET household_id = people.household_id
      FROM people
      WHERE active_storage_attachments.record_type = 'Person'
        AND active_storage_attachments.record_id = people.id
        AND active_storage_attachments.household_id IS NULL
        AND people.household_id IS NOT NULL;
    SQL
  end

  def assert_no_null_household_ids!
    TENANT_TABLES.each do |table_name|
      next unless household_table?(table_name)

      count = select_value(<<~SQL.squish)
        SELECT COUNT(*)
        FROM #{quote_table_name(table_name)}
        WHERE household_id IS NULL
      SQL
      next if count.to_i.zero?

      raise ActiveRecord::IrreversibleMigration,
            "#{table_name} has #{count} rows without household_id; run the household migrator or repair ownership"
    end
  end

  def enforce_household_id_not_null
    TENANT_TABLES.each do |table_name|
      change_column_null table_name, :household_id, false if household_table?(table_name)
    end
  end

  def replace_household_rls_policies
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

  def disable_global_versions_rls
    return unless household_table?(:versions)

    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON versions;'
    execute 'ALTER TABLE versions NO FORCE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE versions DISABLE ROW LEVEL SECURITY;'
  end

  def household_table?(table_name)
    table_exists?(table_name) && column_exists?(table_name, :household_id)
  end
end
