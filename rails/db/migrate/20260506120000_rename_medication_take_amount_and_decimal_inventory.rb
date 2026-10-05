# frozen_string_literal: true

class RenameMedicationTakeAmountAndDecimalInventory < ActiveRecord::Migration[8.1]
  def up
    rename_column :medication_takes, :amount_ml, :dose_amount
    add_column :medication_takes, :dose_unit, :string
    change_column :medication_takes, :dose_amount, :decimal, precision: 10, scale: 2

    change_column :medications, :current_supply, :decimal, precision: 10, scale: 2
    change_column :medications, :reorder_threshold, :decimal, precision: 10, scale: 2, default: 10, null: false
    change_column :medications, :supply_at_last_restock, :decimal, precision: 10, scale: 2

    change_column :dosages, :current_supply, :decimal, precision: 10, scale: 2
    change_column :dosages, :reorder_threshold, :decimal, precision: 10, scale: 2
  end

  def down
    change_column :dosages, :reorder_threshold, :integer
    change_column :dosages, :current_supply, :integer

    change_column :medications, :supply_at_last_restock, :integer
    change_column :medications, :reorder_threshold, :integer, default: 10, null: false
    change_column :medications, :current_supply, :integer

    change_column :medication_takes, :dose_amount, :decimal
    remove_column :medication_takes, :dose_unit
    rename_column :medication_takes, :dose_amount, :amount_ml
  end
end
