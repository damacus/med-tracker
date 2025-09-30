# frozen_string_literal: true

class AddAmountMlToMedicationTakes < ActiveRecord::Migration[8.0]
  def change
    add_column :medication_takes, :amount_ml, :decimal
  end
end
