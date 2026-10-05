# frozen_string_literal: true

class AddScheduleConfigurationToSchedules < ActiveRecord::Migration[8.0]
  def change
    add_column :schedules, :schedule_type, :integer, null: false, default: 0
    add_column :schedules, :schedule_config, :jsonb, null: false, default: {}

    add_index :schedules, :schedule_type
    add_index :schedules, :schedule_config, using: :gin
  end
end
