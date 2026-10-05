# frozen_string_literal: true

# Migration to add details to prescriptions
class AddDetailsToPrescriptions < ActiveRecord::Migration[8.0]
  def change
    change_table :prescriptions, bulk: true do |t|
      t.string :frequency
      t.text :notes
      t.integer :max_daily_doses, default: 4
      t.integer :min_hours_between_doses
      t.integer :dose_cycle
    end
  end
end
