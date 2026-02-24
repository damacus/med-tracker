# frozen_string_literal: true

class RemoveStockFromMedicines < ActiveRecord::Migration[8.0]
  def change
    remove_column :medicines, :stock, :integer
  end
end
