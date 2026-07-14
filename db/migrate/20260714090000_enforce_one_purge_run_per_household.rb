# frozen_string_literal: true

class EnforceOnePurgeRunPerHousehold < ActiveRecord::Migration[8.1]
  def up
    remove_index :household_purge_runs, :household_id
    add_index :household_purge_runs, :household_id, unique: true
  end

  def down
    remove_index :household_purge_runs, :household_id
    add_index :household_purge_runs, :household_id
  end
end
