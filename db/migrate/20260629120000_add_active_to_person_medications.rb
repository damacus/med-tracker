# frozen_string_literal: true

class AddActiveToPersonMedications < ActiveRecord::Migration[8.1]
  def change
    add_column :person_medications, :active, :boolean, default: true, null: false
    add_index :person_medications, :active
  end
end
