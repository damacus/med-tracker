# frozen_string_literal: true

class AddTimestampsToAccounts < ActiveRecord::Migration[8.0]
  def change
    add_timestamps :accounts, null: false, default: -> { 'CURRENT_TIMESTAMP' }
  end
end
