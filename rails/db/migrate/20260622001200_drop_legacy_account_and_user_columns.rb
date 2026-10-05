# frozen_string_literal: true

class DropLegacyAccountAndUserColumns < ActiveRecord::Migration[8.1]
  def change
    remove_column :users, :role, :integer if column_exists?(:users, :role)
    remove_column :accounts, :subscription_plan, :string if column_exists?(:accounts, :subscription_plan)
  end
end
