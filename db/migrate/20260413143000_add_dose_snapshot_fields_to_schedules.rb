# frozen_string_literal: true

class AddDoseSnapshotFieldsToSchedules < ActiveRecord::Migration[8.0]
  def up
    add_column :schedules, :dose_amount, :decimal, precision: 10, scale: 2
    add_column :schedules, :dose_unit, :string

    execute <<~SQL
      UPDATE schedules
      SET dose_amount = dosages.amount,
          dose_unit = dosages.unit
      FROM dosages
      WHERE schedules.dosage_id = dosages.id
    SQL
  end

  def down
    remove_column :schedules, :dose_amount
    remove_column :schedules, :dose_unit
  end
end
