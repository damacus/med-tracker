# frozen_string_literal: true

class AddSupplyAtLastRestockToMedicines < ActiveRecord::Migration[8.0]
  def change
    add_column :medicines, :supply_at_last_restock, :integer
  end
end
