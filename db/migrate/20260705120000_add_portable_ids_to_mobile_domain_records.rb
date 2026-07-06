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
      add_column table_name, :portable_id, :string
      backfill_portable_ids(table_name)
      change_column_null table_name, :portable_id, false
      add_index table_name, %i[household_id portable_id], unique: true
    end
  end

  def down
    TABLES.reverse_each do |table_name|
      remove_index table_name, column: %i[household_id portable_id]
      remove_column table_name, :portable_id
    end
  end

  private

  def backfill_portable_ids(table_name)
    quoted_table = quote_table_name(table_name)
    rows = select_values("SELECT id FROM #{quoted_table} WHERE portable_id IS NULL")

    rows.each do |id|
      execute <<~SQL.squish
        UPDATE #{quoted_table}
        SET portable_id = #{quote(SecureRandom.uuid)}
        WHERE id = #{quote(id)}
      SQL
    end
  end
end
