# frozen_string_literal: true

class AddPerformanceIndexes < ActiveRecord::Migration[8.1]
  def change
    # Audit logs filtering - commonly filtered by whodunnit and event
    add_index :versions, :whodunnit
    add_index :versions, :event
    add_index :versions, :created_at

    # Prescriptions filtering - active is frequently queried
    add_index :prescriptions, :active

    # Medication takes - taken_at is used for sorting and filtering
    add_index :medication_takes, :taken_at
  end
end
