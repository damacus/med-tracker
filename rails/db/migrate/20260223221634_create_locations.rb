# frozen_string_literal: true

class CreateLocations < ActiveRecord::Migration[8.1]
  def change
    create_table :locations do |t|
      t.string :name, null: false
      t.text :description

      t.timestamps
    end

    add_index :locations, :name, unique: true
  end
end
