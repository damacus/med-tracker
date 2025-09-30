# frozen_string_literal: true

class CreateMedicines < ActiveRecord::Migration[8.0]
  def change
    create_table :medicines do |t|
      t.string :name
      t.integer :current_supply

      t.timestamps
    end
  end
end
