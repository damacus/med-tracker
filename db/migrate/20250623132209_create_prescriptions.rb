# frozen_string_literal: true

class CreatePrescriptions < ActiveRecord::Migration[8.0]
  def change
    create_table :prescriptions do |t|
      t.references :user, null: false, foreign_key: true
      t.references :medicine, null: false, foreign_key: true
      t.references :dosage, null: false, foreign_key: true
      t.date :start_date
      t.date :end_date
      t.boolean :active, default: true, null: false

      t.timestamps
    end
  end
end
