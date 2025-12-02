# frozen_string_literal: true

# Migration for Rodauth lockout feature
# Creates tables to track failed login attempts and account lockouts
class CreateAccountLockouts < ActiveRecord::Migration[8.1]
  def change
    # Track failed login attempts per account
    create_table :account_login_failures, id: false do |t|
      t.bigint :account_id, null: false, primary_key: true
      t.integer :number, null: false, default: 1
      t.timestamps
    end

    add_index :account_login_failures, :account_id

    # Track account lockouts with deadline
    create_table :account_lockouts, id: false do |t|
      t.bigint :account_id, null: false, primary_key: true
      t.string :key, null: false
      t.datetime :deadline, null: false
      t.datetime :email_last_sent
      t.timestamps
    end

    add_index :account_lockouts, :account_id
  end
end
