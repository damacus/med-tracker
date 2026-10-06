# frozen_string_literal: true

class AddGlobalSearchTrigramIndexes < ActiveRecord::Migration[8.0]
  def change
    enable_extension 'pg_trgm' unless extension_enabled?('pg_trgm')

    add_index :people, :name, using: :gin, opclass: :gin_trgm_ops, name: 'index_people_on_name_trigram'
    add_index :medications, :name, using: :gin, opclass: :gin_trgm_ops, name: 'index_medications_on_name_trigram'
    add_index :medications, :category, using: :gin, opclass: :gin_trgm_ops, name: 'index_medications_on_category_trigram'
    add_index :medications, :barcode, using: :gin, opclass: :gin_trgm_ops, name: 'index_medications_on_barcode_trigram'
    add_index :medications, :dmd_code, using: :gin, opclass: :gin_trgm_ops, name: 'index_medications_on_dmd_code_trigram'
    add_index :locations, :name, using: :gin, opclass: :gin_trgm_ops, name: 'index_locations_on_name_trigram'
  end
end
