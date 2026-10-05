# frozen_string_literal: true

# Fix account_login_failures table - Rodauth doesn't set timestamps
# Make created_at and updated_at nullable since Sequel doesn't populate them
class FixAccountLockoutTimestamps < ActiveRecord::Migration[8.1]
  def change
    change_table :account_login_failures, bulk: true do |t|
      t.change_null :created_at, true
      t.change_null :updated_at, true
    end

    change_table :account_lockouts, bulk: true do |t|
      t.change_null :created_at, true
      t.change_null :updated_at, true
    end
  end
end
