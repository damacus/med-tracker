# frozen_string_literal: true

class AddPersonTypeAndCapacityToPeople < ActiveRecord::Migration[8.0]
  def change
    change_table :people, bulk: true do |t|
      t.integer :person_type, default: 0, null: false
      t.boolean :has_capacity, default: true, null: false
    end

    add_index :people, :person_type
  end
end
