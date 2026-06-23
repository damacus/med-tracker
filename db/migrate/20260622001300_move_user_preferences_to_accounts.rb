# frozen_string_literal: true

class MoveUserPreferencesToAccounts < ActiveRecord::Migration[8.1]
  def up
    add_column :accounts, :preferences, :jsonb, null: false, default: {} unless column_exists?(:accounts, :preferences)
    add_index :accounts, :preferences, using: :gin unless index_exists?(:accounts, :preferences)

    execute <<~SQL.squish
      UPDATE accounts
      SET preferences = COALESCE(accounts.preferences, '{}'::jsonb) || COALESCE(users.preferences, '{}'::jsonb)
      FROM people
      INNER JOIN users ON users.person_id = people.id
      WHERE people.account_id = accounts.id
        AND COALESCE(users.preferences, '{}'::jsonb) <> '{}'::jsonb
    SQL

    remove_index :users, :preferences if index_exists?(:users, :preferences)
    remove_column :users, :preferences if column_exists?(:users, :preferences)
  end

  def down
    add_column :users, :preferences, :jsonb, null: false, default: {} unless column_exists?(:users, :preferences)
    add_index :users, :preferences, using: :gin unless index_exists?(:users, :preferences)

    execute <<~SQL.squish
      UPDATE users
      SET preferences = COALESCE(users.preferences, '{}'::jsonb) || COALESCE(accounts.preferences, '{}'::jsonb)
      FROM people
      INNER JOIN accounts ON accounts.id = people.account_id
      WHERE users.person_id = people.id
        AND COALESCE(accounts.preferences, '{}'::jsonb) <> '{}'::jsonb
    SQL

    remove_index :accounts, :preferences if index_exists?(:accounts, :preferences)
    remove_column :accounts, :preferences if column_exists?(:accounts, :preferences)
  end
end
