# frozen_string_literal: true

class RestoreFrequencyOnSchedules < ActiveRecord::Migration[8.1]
  def change
    add_column :schedules, :frequency, :string
  end
end
