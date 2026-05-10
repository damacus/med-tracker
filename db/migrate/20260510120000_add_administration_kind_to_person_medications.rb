# frozen_string_literal: true

class AddAdministrationKindToPersonMedications < ActiveRecord::Migration[8.1]
  def up
    add_column :person_medications, :administration_kind, :integer, default: 1, null: false
    add_index :person_medications, :administration_kind

    execute <<~SQL.squish
      UPDATE person_medications
      SET administration_kind = 0,
          min_hours_between_doses = CASE
            WHEN max_daily_doses = 1 AND min_hours_between_doses = 24 THEN NULL
            ELSE min_hours_between_doses
          END
      WHERE COALESCE(dose_cycle, 0) = 0
        AND max_daily_doses IS NOT NULL
        AND (
          min_hours_between_doses IS NULL OR
          (max_daily_doses = 1 AND min_hours_between_doses = 24)
        )
    SQL

    execute <<~SQL.squish
      UPDATE schedules
      SET schedule_type = 4
      WHERE schedule_type <> 4
        AND (
          LOWER(COALESCE(frequency, '')) = 'as needed' OR
          schedule_config @> '{"as_needed": true}'::jsonb
        )
    SQL
  end

  def down
    remove_index :person_medications, :administration_kind
    remove_column :person_medications, :administration_kind
  end
end
