# frozen_string_literal: true

class RemoveDosageIdFromSchedules < ActiveRecord::Migration[8.1]
  def change
    remove_foreign_key :schedules, :dosages
    remove_index :schedules, :dosage_id
    remove_column :schedules, :dosage_id, :bigint
  end
end
