# frozen_string_literal: true

class AllowAccountLinkedPersonRlsLoginLookup < ActiveRecord::Migration[8.1]
  def up
    return unless table_exists?(:people) && column_exists?(:people, :account_id)
    return unless runtime_role_exists?('med_tracker_app')

    execute 'DROP POLICY IF EXISTS people_account_login_lookup ON people;'
    execute <<~SQL
      CREATE POLICY people_account_login_lookup ON people
      FOR SELECT TO med_tracker_app
      USING (account_id IS NOT NULL);
    SQL
  end

  def down
    execute 'DROP POLICY IF EXISTS people_account_login_lookup ON people;' if table_exists?(:people)
  end

  private

  def runtime_role_exists?(role_name)
    select_value(<<~SQL.squish).to_i.positive?
      SELECT COUNT(*)
      FROM pg_roles
      WHERE rolname = #{quote(role_name)}
    SQL
  end
end
