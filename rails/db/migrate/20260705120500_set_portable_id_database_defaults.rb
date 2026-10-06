# frozen_string_literal: true

class SetPortableIdDatabaseDefaults < ActiveRecord::Migration[8.1]
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
      change_column_default table_name, :portable_id, from: nil, to: -> { 'gen_random_uuid()::text' }
    end
  end

  def down
    TABLES.each do |table_name|
      change_column_default table_name, :portable_id, from: -> { 'gen_random_uuid()::text' }, to: nil
    end
  end
end
