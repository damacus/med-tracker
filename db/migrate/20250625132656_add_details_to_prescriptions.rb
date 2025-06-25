# frozen_string_literal: true

# Migration to add details to prescriptions
class AddDetailsToPrescriptions < ActiveRecord::Migration[8.0]
  def change
    add_column :prescriptions, :frequency, :string
    add_column :prescriptions, :notes, :text
    add_column :prescriptions, :max_daily_doses, :integer, default: 4
    add_column :prescriptions, :min_hours_between_doses, :integer
    add_column :prescriptions, :dose_cycle, :integer
  end
end
