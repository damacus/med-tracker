# frozen_string_literal: true

class AddPersonTypeAndCapacityToPeople < ActiveRecord::Migration[8.0]
  def change
    add_column :people, :person_type, :integer, default: 0, null: false
    add_column :people, :has_capacity, :boolean, default: true, null: false

    add_index :people, :person_type
  end
end
