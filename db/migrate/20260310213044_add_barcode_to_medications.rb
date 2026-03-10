# frozen_string_literal: true

class AddBarcodeToMedications < ActiveRecord::Migration[8.0]
  def change
    add_column :medications, :barcode, :string
    add_index :medications, :barcode, unique: true, where: "barcode IS NOT NULL AND barcode != ''"
  end
end
