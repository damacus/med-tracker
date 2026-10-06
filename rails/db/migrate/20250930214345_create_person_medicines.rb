# frozen_string_literal: true

class CreatePersonMedicines < ActiveRecord::Migration[8.0]
  def change
    create_table :person_medicines do |t|
      t.references :person, null: false, foreign_key: true
      t.references :medicine, null: false, foreign_key: true
      t.text :notes
      t.integer :max_daily_doses
      t.integer :min_hours_between_doses

      t.timestamps
    end
  end
end
