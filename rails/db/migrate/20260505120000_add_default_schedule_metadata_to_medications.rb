# frozen_string_literal: true

class AddDefaultScheduleMetadataToMedications < ActiveRecord::Migration[8.0]
  def up
    add_column :medications, :default_schedule_type, :integer, null: false, default: 1
    add_column :medications, :default_schedule_config, :jsonb, null: false, default: {}
    add_index :medications, :default_schedule_type

    execute <<~SQL.squish
      UPDATE medications
      SET default_schedule_type = CASE
        WHEN EXISTS (
          SELECT 1
          FROM dosages
          WHERE dosages.medication_id = medications.id
          AND lower(trim(dosages.frequency)) = 'as needed'
        ) THEN 4
        ELSE 1
      END
    SQL
  end

  def down
    remove_index :medications, :default_schedule_type
    remove_column :medications, :default_schedule_config
    remove_column :medications, :default_schedule_type
  end
end
