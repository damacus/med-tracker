# frozen_string_literal: true

class CreateAccountLockouts < ActiveRecord::Migration[8.1]
  def change
    create_table :account_login_failures, id: false do |t|
      t.bigint :account_id, null: false, primary_key: true
      t.integer :number, null: false, default: 1
      t.timestamps
    end

    add_index :account_login_failures, :account_id
    add_foreign_key :account_login_failures, :accounts, column: :account_id

    create_table :account_lockouts, id: false do |t|
      t.bigint :account_id, null: false, primary_key: true
      t.string :key, null: false
      t.datetime :deadline, null: false
      t.datetime :email_last_sent
      t.timestamps
    end

    add_index :account_lockouts, :account_id
    add_foreign_key :account_lockouts, :accounts, column: :account_id
  end
end
