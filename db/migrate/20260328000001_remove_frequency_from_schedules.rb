# frozen_string_literal: true

class RemoveFrequencyFromSchedules < ActiveRecord::Migration[8.1]
  def change
    remove_column :schedules, :frequency, :string
  end
end
