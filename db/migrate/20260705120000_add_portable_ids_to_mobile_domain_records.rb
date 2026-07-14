# frozen_string_literal: true

class AddPortableIdsToMobileDomainRecords < ActiveRecord::Migration[8.1]
  TABLES = %i[
    people
    locations
    medications
    dosages
    schedules
    person_medications
    medication_takes
    notification_preferences
  ].freeze

  def up
    TABLES.each do |table_name|
      add_column table_name, :portable_id, :string unless column_exists?(table_name, :portable_id)
      prepare_portable_ids(table_name)
      change_column_null table_name, :portable_id, false
      add_index table_name, %i[household_id portable_id], unique: true unless
        index_exists?(table_name, %i[household_id portable_id], unique: true)
    end
  end

  def down
    TABLES.reverse_each do |table_name|
      remove_index table_name, column: %i[household_id portable_id]
      remove_column table_name, :portable_id
    end
  end

  private

  def prepare_portable_ids(table_name)
    with_forced_rls_relaxed(table_name) do
      backfill_portable_ids(table_name)
      verify_no_null_portable_ids!(table_name)
    end
  end

  def backfill_portable_ids(table_name)
    execute <<~SQL.squish
      UPDATE #{quote_table_name(table_name)}
      SET portable_id = gen_random_uuid()::text
      WHERE portable_id IS NULL
    SQL
  end

  def verify_no_null_portable_ids!(table_name)
    count = select_value(<<~SQL.squish).to_i
      SELECT COUNT(*)
      FROM #{quote_table_name(table_name)}
      WHERE portable_id IS NULL
    SQL
    return if count.zero?

    raise ActiveRecord::IrreversibleMigration,
          "#{table_name} has #{count} rows without portable_id"
  end

  def with_forced_rls_relaxed(table_name)
    forced = forced_row_level_security?(table_name)
    quoted_table = quote_table_name(table_name)
    execute "ALTER TABLE #{quoted_table} NO FORCE ROW LEVEL SECURITY" if forced
    yield
  ensure
    execute "ALTER TABLE #{quoted_table} FORCE ROW LEVEL SECURITY" if forced
  end

  def forced_row_level_security?(table_name)
    select_value(<<~SQL.squish)
      SELECT relforcerowsecurity
      FROM pg_class
      WHERE oid = #{quote(table_name)}::regclass
    SQL
  end
end
