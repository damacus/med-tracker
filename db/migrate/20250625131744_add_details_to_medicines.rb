# frozen_string_literal: true

class AddDetailsToMedicines < ActiveRecord::Migration[8.0]
  def change
    change_table :medicines, bulk: true do |m|
      m.float :dosage_amount
      m.string :dosage_unit
      m.integer :stock
      m.date :expiry_date
      m.text :description
      m.text :warnings
    end
  end
end
