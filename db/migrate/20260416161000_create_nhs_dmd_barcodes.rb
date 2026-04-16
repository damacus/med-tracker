# frozen_string_literal: true

class CreateNhsDmdBarcodes < ActiveRecord::Migration[8.1]
  def change
    create_table :nhs_dmd_barcodes do |t|
      t.string :gtin, null: false
      t.string :code, null: false
      t.string :display, null: false
      t.string :system, null: false, default: 'https://dmd.nhs.uk'
      t.string :concept_class
      t.timestamps
    end

    add_index :nhs_dmd_barcodes, :gtin, unique: true
    add_index :nhs_dmd_barcodes, :code
  end
end
