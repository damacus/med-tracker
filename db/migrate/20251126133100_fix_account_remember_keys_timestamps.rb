# frozen_string_literal: true

class FixAccountRememberKeysTimestamps < ActiveRecord::Migration[8.0]
  def change
    change_table :account_remember_keys, bulk: true do |t|
      t.change_null :created_at, true
      t.change_null :updated_at, true
    end
  end
end
