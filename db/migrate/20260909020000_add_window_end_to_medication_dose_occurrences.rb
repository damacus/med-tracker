class AddWindowEndToMedicationDoseOccurrences < ActiveRecord::Migration[8.1]
  def up
    add_column :medication_dose_occurrences, :window_ends_on, :date
    backfill_windows
    add_check_constraint :medication_dose_occurrences, 'window_ends_on >= window_starts_on',
                         name: 'chk_dose_occurrences_window_order'
  end

  def down
    remove_check_constraint :medication_dose_occurrences, name: 'chk_dose_occurrences_window_order'
    remove_column :medication_dose_occurrences, :window_ends_on
  end

  private

  def backfill_windows
    previous = connection.select_value("SELECT current_setting('med_tracker.current_household_id', true)")
    connection.select_values('SELECT id FROM households ORDER BY id').each do |household_id|
      set_household_context(household_id)
      backfill_household(household_id)
    end
  ensure
    set_household_context(previous)
  end

  def set_household_context(household_id)
    execute "SELECT set_config('med_tracker.current_household_id', #{quote(household_id.to_s)}, true)"
  end

  def backfill_household(household_id)
    execute <<~SQL.squish
      UPDATE medication_dose_occurrences AS outcomes
      SET window_ends_on = CASE
        WHEN schedule_id IS NOT NULL THEN window_starts_on
        ELSE CASE (SELECT dose_cycle FROM person_medications WHERE id = outcomes.person_medication_id)
          WHEN 1 THEN window_starts_on + 6
          WHEN 2 THEN (date_trunc('month', window_starts_on) + INTERVAL '1 month - 1 day')::date
          ELSE window_starts_on
        END
      END
      WHERE household_id = #{quote(household_id)}
    SQL
  end
end
