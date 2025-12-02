# frozen_string_literal: true

# Migration for Rodauth active_sessions feature
# Tracks active sessions per account for session management
class CreateAccountActiveSessionKeys < ActiveRecord::Migration[8.1]
  def change
    create_table :account_active_session_keys, id: false do |t|
      t.bigint :account_id, null: false
      t.string :session_id, null: false
      t.datetime :created_at, null: false, default: -> { 'CURRENT_TIMESTAMP' }
      t.datetime :last_use, null: false, default: -> { 'CURRENT_TIMESTAMP' }
    end

    add_index :account_active_session_keys, %i[account_id session_id], unique: true
    add_index :account_active_session_keys, :account_id
  end
end
