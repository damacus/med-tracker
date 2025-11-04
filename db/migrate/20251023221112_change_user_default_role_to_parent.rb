# frozen_string_literal: true

class ChangeUserDefaultRoleToParent < ActiveRecord::Migration[8.0]
  def up
    # Change default from 0 (administrator) to 4 (parent)
    change_column_default :users, :role, from: 0, to: 4
  end

  def down
    # Revert back to administrator default if needed
    change_column_default :users, :role, from: 4, to: 0
  end
end
