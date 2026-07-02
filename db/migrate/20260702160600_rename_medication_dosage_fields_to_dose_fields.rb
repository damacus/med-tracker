# frozen_string_literal: true

class RenameMedicationDosageFieldsToDoseFields < ActiveRecord::Migration[8.1]
  def change
    rename_column :medications, :dosage_amount, :dose_amount
    rename_column :medications, :dosage_unit, :dose_unit
  end
end
