# frozen_string_literal: true

class AddInventoryFieldsToDosages < ActiveRecord::Migration[8.1]
  def change
    add_column :dosages, :current_supply, :integer
    add_column :dosages, :reorder_threshold, :integer
  end
end
