# frozen_string_literal: true

class AddDmdFieldsToMedications < ActiveRecord::Migration[8.1]
  def change
    add_column :medications, :dmd_code, :string
    add_column :medications, :dmd_system, :string
    add_column :medications, :dmd_concept_class, :string

    add_index :medications, :dmd_code
  end
end
