# frozen_string_literal: true

module MedicationAdministration
  class HistoricalDataMigration
    def backfill_household(household:)
      MedicationTake.where(household_id: nil).find_each do |take|
        write_column(take, :household_id, household.id)
      end
    end

    def move_location(from:, into:)
      MedicationTake.where(taken_from_location: from).find_each do |take|
        write_column(take, :taken_from_location_id, into.id)
      end
    end

    private

    def write_column(record, column_name, value)
      connection = record.class.connection
      table_name = connection.quote_table_name(record.class.table_name)
      column = connection.quote_column_name(column_name)
      sql = record.class.sanitize_sql_array(["UPDATE #{table_name} SET #{column} = ? WHERE id = ?", value, record.id])
      connection.execute(sql)
      record[column_name] = value
    end
  end
end
