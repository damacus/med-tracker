# frozen_string_literal: true

class CreateBarcodeCatalogEntries < ActiveRecord::Migration[8.1]
  def change
    create_table :barcode_catalog_entries do |t|
      t.string :gtin, null: false
      t.string :display, null: false
      t.string :source, null: false
      t.string :code
      t.string :system
      t.string :concept_class
      t.timestamps
    end

    add_index :barcode_catalog_entries, %i[source gtin], unique: true
    add_index :barcode_catalog_entries, :gtin
  end
end
