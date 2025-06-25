# frozen_string_literal: true

# Migration to add active column to prescriptions
class AddActiveToPresciptions < ActiveRecord::Migration[8.0]
  def change
    add_column :prescriptions, :active, :boolean, default: true
  end
end
