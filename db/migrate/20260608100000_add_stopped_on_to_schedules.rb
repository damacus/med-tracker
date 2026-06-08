# frozen_string_literal: true

class AddStoppedOnToSchedules < ActiveRecord::Migration[8.1]
  def up
    add_column :schedules, :stopped_on, :date unless column_exists?(:schedules, :stopped_on)
    add_index :schedules, :stopped_on unless index_exists?(:schedules, :stopped_on)
  end

  def down
    remove_index :schedules, :stopped_on if index_exists?(:schedules, :stopped_on)
    remove_column :schedules, :stopped_on if column_exists?(:schedules, :stopped_on)
  end
end
